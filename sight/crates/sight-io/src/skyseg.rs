//! The sky-segmentation network: U²-Net from Sky-Segmentation-and-Post-processing
//! (xiongzhu666), as `skyseg.onnx` from Hugging Face `JianyuanWang/skyseg` (MIT), run on the
//! CPU by tract. It takes the photo squeezed to 320 × 320 RGB with `ImageNet` normalisation and
//! returns one value per cell, higher for sky; VGGT and others use it the same way.

use std::path::{Path, PathBuf};

use sight_core::{sky::Image, skymap::SkyMap};
use tract_onnx::prelude::*;

use crate::IoError;

/// The network's input and output size.
const SIZE: usize = 320;
const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const STD: [f32; 3] = [0.229, 0.224, 0.225];

type Plan = std::sync::Arc<TypedRunnableModel>;

/// The loaded, optimised network.
pub struct SkyModel {
    plan: Plan,
    path: PathBuf,
}

impl SkyModel {
    /// Loads and optimises the network; takes a few seconds, once.
    pub fn load(path: &Path) -> Result<Self, IoError> {
        let err = |e: TractError| IoError::Model {
            path: path.to_path_buf(),
            message: format!("{e:#}"),
        };
        let plan = tract_onnx::onnx()
            .model_for_path(path)
            .map_err(err)?
            .with_input_fact(0, f32::fact([1, 3, SIZE, SIZE]).into())
            .map_err(err)?
            .into_optimized()
            .map_err(err)?
            .into_runnable()
            .map_err(err)?;
        Ok(Self {
            plan,
            path: path.to_path_buf(),
        })
    }

    /// How sky-like each part of `img` is, 0 to 1, on a 320 × 320 grid over the frame.
    pub fn sky(&self, img: &Image) -> Result<SkyMap, IoError> {
        let err = |e: TractError| IoError::Model {
            path: self.path.clone(),
            message: format!("{e:#}"),
        };
        let input = squeeze(img);
        let tensor: Tensor = tract_ndarray::Array4::from_shape_vec((1, 3, SIZE, SIZE), input)
            .map_err(|e| err(e.into()))?
            .into();
        let out = self.plan.run(tvec!(tensor.into())).map_err(err)?;
        let first = out
            .first()
            .ok_or_else(|| err(TractError::msg("no output")))?;
        let view = first.to_plain_array_view::<f32>().map_err(err)?;
        let mut p: Vec<f32> = view.iter().copied().collect();
        if p.len() != SIZE * SIZE {
            return Err(err(TractError::msg(format!(
                "output has {} values, not {}",
                p.len(),
                SIZE * SIZE
            ))));
        }
        // Stretched to 0..1, as the reference post-processing does.
        let (lo, hi) = p
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &v| {
                (a.min(v), b.max(v))
            });
        let span = (hi - lo).max(1e-6);
        for v in &mut p {
            *v = (*v - lo) / span;
        }
        Ok(SkyMap {
            width: SIZE,
            height: SIZE,
            p,
        })
    }
}

/// The photo averaged down to SIZE × SIZE, as normalised RGB planes (NCHW).
fn squeeze(img: &Image) -> Vec<f32> {
    let mut out = vec![0.0_f32; 3 * SIZE * SIZE];
    for cy in 0..SIZE {
        let y0 = cy * img.height / SIZE;
        let y1 = ((cy + 1) * img.height / SIZE).max(y0 + 1).min(img.height);
        for cx in 0..SIZE {
            let x0 = cx * img.width / SIZE;
            let x1 = ((cx + 1) * img.width / SIZE).max(x0 + 1).min(img.width);
            let mut sum = [0.0_f64; 3];
            for y in y0..y1 {
                for &px in &img.argb[y * img.width + x0..y * img.width + x1] {
                    sum[0] += f64::from((px >> 16) & 0xff);
                    sum[1] += f64::from((px >> 8) & 0xff);
                    sum[2] += f64::from(px & 0xff);
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as f64;
            for ch in 0..3 {
                let v = (sum[ch] / n / 255.0) as f32;
                out[ch * SIZE * SIZE + cy * SIZE + cx] = (v - MEAN[ch]) / STD[ch];
            }
        }
    }
    out
}
