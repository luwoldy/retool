//! Engine R: reference-set ("realm") embedding scorer.
//! DINOv2-reg4 embeddings + cosine kNN distance, conformal-calibrated
//! against the reference set (recipe per RESEARCH-composition.md §1).
use anyhow::{bail, Context, Result};
use candle_core::{DType, Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::dino;

const WEIGHTS_REPO: &str = "timm/vit_base_patch14_reg4_dinov2.lvd142m";
const IMAGENET_MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const IMAGENET_STD: [f32; 3] = [0.229, 0.224, 0.225];

#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub path: String,
    pub label: String,
    pub emb: Vec<f32>,
}

#[derive(Serialize, Deserialize)]
pub struct Realm {
    pub k: usize,
    pub entries: Vec<Entry>,
    /// sorted leave-one-out kNN scores of the references (conformal baseline)
    pub loo: Vec<f32>,
    /// distance above which "no similar reference" (p99 of LOO 1-NN distances)
    pub no_ref_threshold: f32,
}

pub struct Embedder {
    model: dino::DinoVisionTransformer,
    device: Device,
}

impl Embedder {
    pub fn new() -> Result<Self> {
        let device = Device::new_metal(0).unwrap_or(Device::Cpu);
        eprintln!("device: {}", if device.is_metal() { "metal" } else { "cpu" });
        let cache = std::env::home_dir().context("no home dir")?.join(".cache/retool");
        std::fs::create_dir_all(&cache)?;
        let weights = cache.join("dinov2-reg4-base.safetensors");
        if !weights.exists() {
            eprintln!("downloading DINOv2 weights (~330MB, one-time)...");
            let url = format!("https://huggingface.co/{WEIGHTS_REPO}/resolve/main/model.safetensors");
            let tmp = weights.with_extension("part");
            let ok = std::process::Command::new("curl")
                .args(["-L", "--fail", "-o"])
                .arg(&tmp)
                .arg(&url)
                .status()
                .context("run curl")?
                .success();
            if !ok {
                bail!("weight download failed: {url}");
            }
            std::fs::rename(&tmp, &weights)?;
        }
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights], DType::F32, &device)? };
        let model = dino::vit_base(vb)?;
        Ok(Self { model, device })
    }

    /// Embed one image: CLS token ++ mean of patch tokens, L2-normalized.
    /// The full frame is squash-resized to 518x518 (no crop) so composition
    /// at the frame edges survives; distortion is identical for refs and queries.
    pub fn embed(&self, path: &Path) -> Result<Vec<f32>> {
        let sz = dino::IMG_SIZE as u32;
        let img = image::ImageReader::open(path)?
            .decode()
            .with_context(|| format!("decode {}", path.display()))?
            .to_rgb8();
        let img = image::imageops::resize(&img, sz, sz, image::imageops::FilterType::Triangle);
        let n = (sz * sz) as usize;
        let mut data = vec![0f32; 3 * n];
        for (x, y, p) in img.enumerate_pixels() {
            for c in 0..3 {
                data[c * n + (y * sz + x) as usize] = (p[c] as f32 / 255.0 - IMAGENET_MEAN[c]) / IMAGENET_STD[c];
            }
        }
        let t = Tensor::from_vec(data, (1, 3, sz as usize, sz as usize), &self.device)?;
        let tokens = self.model.forward_features(&t)?;
        let cls = tokens.i((0, 0))?;
        let patch_mean = tokens.i((0, dino::N_SPECIAL_TOKENS..))?.mean(0)?;
        let mut emb: Vec<f32> = Tensor::cat(&[&cls, &patch_mean], 0)?.to_vec1()?;
        let norm = emb.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-12);
        emb.iter_mut().for_each(|v| *v /= norm);
        Ok(emb)
    }
}

fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    1.0 - a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>()
}

/// Mean of the k smallest distances (sorts in place).
fn knn_mean(dists: &mut Vec<f32>, k: usize) -> f32 {
    dists.sort_by(|a, b| a.total_cmp(b));
    let k = k.min(dists.len());
    dists[..k].iter().sum::<f32>() / k as f32
}

fn image_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in walk(dir) {
        let ext = entry.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if ["jpg", "jpeg", "png", "webp"].contains(&ext.as_str()) {
            out.push(entry);
        }
    }
    out.sort();
    out
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}

pub fn build(refs_dir: &Path, out_path: &Path) -> Result<()> {
    let files = image_files(refs_dir);
    if files.len() < 10 {
        bail!("only {} images under {} — need at least 10", files.len(), refs_dir.display());
    }
    let embedder = Embedder::new()?;
    let mut entries = Vec::new();
    for (i, f) in files.iter().enumerate() {
        let label = f
            .parent()
            .filter(|p| *p != refs_dir)
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unlabeled".into());
        eprintln!("[{}/{}] {}", i + 1, files.len(), f.display());
        entries.push(Entry {
            path: f.display().to_string(),
            label,
            emb: embedder.embed(f)?,
        });
    }

    let n = entries.len();
    let k = (n / 20).clamp(3, 20);
    // leave-one-out kNN scores + 1-NN distances
    let mut loo = Vec::with_capacity(n);
    let mut nn1 = Vec::with_capacity(n);
    for i in 0..n {
        let mut dists: Vec<f32> = (0..n)
            .filter(|&j| j != i)
            .map(|j| cosine_distance(&entries[i].emb, &entries[j].emb))
            .collect();
        let score = knn_mean(&mut dists, k);
        nn1.push(dists[0]);
        loo.push(score);
    }
    let mut nn1_sorted = nn1.clone();
    nn1_sorted.sort_by(|a, b| a.total_cmp(b));
    let no_ref_threshold = nn1_sorted[(n * 99 / 100).min(n - 1)] * 1.5;
    let mut loo_sorted = loo.clone();
    loo_sorted.sort_by(|a, b| a.total_cmp(b));

    // profile report: per-label counts and tightness
    println!("realm: {n} references, k={k}");
    let mut labels: Vec<String> = entries.iter().map(|e| e.label.clone()).collect();
    labels.sort();
    labels.dedup();
    for l in &labels {
        let idx: Vec<usize> = (0..n).filter(|&i| entries[i].label == *l).collect();
        let mean_loo = idx.iter().map(|&i| loo[i]).sum::<f32>() / idx.len() as f32;
        println!("  {l:<14} {:>3} refs   tightness {:.3} (lower = more consistent)", idx.len(), mean_loo);
    }

    let realm = Realm { k, entries, loo: loo_sorted, no_ref_threshold };
    std::fs::write(out_path, serde_json::to_vec(&realm)?)?;
    println!("wrote {}", out_path.display());
    Ok(())
}

pub fn load(realm_path: &Path) -> Result<Realm> {
    Ok(serde_json::from_slice(&std::fs::read(realm_path).context("read realm file")?)?)
}

pub struct Scored {
    pub path: PathBuf,
    /// 0–100: closer to the references than this % of references are to each other
    pub fit: f32,
    pub dist: f32,
    pub label: String,
    pub nearest: String,
    pub no_ref: bool,
}

pub fn score_files(realm: &Realm, embedder: &Embedder, files: &[PathBuf]) -> Result<Vec<Scored>> {
    let n = realm.loo.len();
    let mut rows = Vec::new();
    for f in files {
        let emb = embedder.embed(f)?;
        let mut dists: Vec<(f32, usize)> = realm
            .entries
            .iter()
            .enumerate()
            .map(|(j, e)| (cosine_distance(&emb, &e.emb), j))
            .collect();
        dists.sort_by(|a, b| a.0.total_cmp(&b.0));
        let s = dists[..realm.k.min(dists.len())].iter().map(|d| d.0).sum::<f32>() / realm.k as f32;
        // conformal: fraction of reference LOO scores >= s (higher = more typical)
        let ge = realm.loo.iter().filter(|&&l| l >= s).count();
        let fit = 100.0 * (1 + ge) as f32 / (n + 1) as f32;
        let nearest = &realm.entries[dists[0].1];
        rows.push(Scored {
            path: f.clone(),
            fit,
            dist: s,
            label: nearest.label.clone(),
            nearest: nearest.path.clone(),
            no_ref: dists[0].0 > realm.no_ref_threshold,
        });
    }
    Ok(rows)
}

pub fn score(realm_path: &Path, target: &Path) -> Result<()> {
    let realm = load(realm_path)?;
    let files = if target.is_dir() { image_files(target) } else { vec![target.to_path_buf()] };
    if files.is_empty() {
        bail!("no images at {}", target.display());
    }
    let embedder = Embedder::new()?;
    let mut rows = score_files(&realm, &embedder, &files)?;
    rows.sort_by(|a, b| b.fit.total_cmp(&a.fit));

    println!("{:>5}  {:<14} {:<40} nearest ref", "fit", "nearest type", "image");
    for Scored { path: f, fit, dist: s, label, nearest, no_ref } in &rows {
        let flag = if *no_ref { "  [no similar reference]" } else { "" };
        let name: String = f
            .components()
            .rev()
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let nname = Path::new(nearest).file_name().unwrap_or_default().to_string_lossy();
        println!("{fit:>5.0}  {label:<14} {name:<40} {nname} (d={s:.3}){flag}");
    }
    Ok(())
}
