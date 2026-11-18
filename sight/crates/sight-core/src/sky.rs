//! The skyline of a photograph: one pixel per column where the land meets the sky. Trees, a
//! pole and a bright cloud are not the skyline. Works on decoded ARGB pixels; decoding is the
//! caller's job.

use crate::camera::Pixel;
use crate::config::SkyConfig;

/// Decoded pixels, row-major ARGB.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub argb: Vec<u32>,
}

#[inline]
fn channels(p: u32) -> (i64, i64, i64) {
    (i64::from((p >> 16) & 0xff), i64::from((p >> 8) & 0xff), i64::from(p & 0xff))
}

#[inline]
fn is_sky(r: i64, g: i64, b: i64) -> bool {
    b > r + 12 && b >= g - 8 && b > 140
}

/// Bright and nearly colourless: pale sky or a cloud edge. A sunlit cliff is redder.
#[inline]
fn is_haze(r: i64, g: i64, b: i64) -> bool {
    let maxv = r.max(g).max(b);
    let minv = r.min(g).min(b);
    let sat = if maxv == 0 { 0.0 } else { (maxv - minv) as f64 / maxv as f64 };
    let lum = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
    lum > 155.0 && sat < 0.18
}

/// The row where column `x` meets land and does not return to sky within `rock_run` pixels.
fn ridge_row(img: &Image, x: usize, rock_run: usize) -> Option<usize> {
    let at = |y: usize| channels(img.argb[y * img.width + x]);
    let mut y = 0;
    while y + rock_run < img.height {
        let (r, g, b) = at(y);
        if is_sky(r, g, b) || is_haze(r, g, b) {
            y += 1;
            continue;
        }
        let back = (1..=rock_run).any(|k| {
            let (r, g, b) = at(y + k);
            is_sky(r, g, b)
        });
        if !back {
            return Some(y);
        }
        y += 1;
    }
    None
}

fn median_of(mut ys: Vec<usize>) -> usize {
    ys.sort_unstable();
    ys[ys.len() / 2]
}

/// Each column replaced by the median of the rows within `radius`, ignoring gaps, when at least
/// `radius` of them are present. A thin spike loses the vote; a gap takes the ridge beside it.
fn fill_median(rows: &[Option<usize>], radius: usize) -> Vec<Option<usize>> {
    let n = rows.len() as i64;
    let r = radius as i64;
    (0..n)
        .map(|x| {
            let w: Vec<usize> = (-r..=r)
                .filter_map(|k| {
                    let xx = x + k;
                    if (0..n).contains(&xx) { rows[xx as usize] } else { None }
                })
                .collect();
            (w.len() >= radius).then(|| median_of(w))
        })
        .collect()
}

/// Drops a column more than `jump` pixels above the wide median: a pole, not a summit.
fn despike(rows: &[Option<usize>], radius: usize, jump: usize) -> Vec<Option<usize>> {
    let wide = fill_median(rows, radius);
    rows.iter()
        .zip(wide)
        .map(|(&y, w)| match (y, w) {
            (None, _) => None,
            (Some(y), None) => Some(y),
            (Some(y), Some(w)) => if (y as i64) < w as i64 - jump as i64 { None } else { Some(y) },
        })
        .collect()
}

/// Skyline pixels in image coordinates. Columns whose ridge is in the top 15% of the frame or
/// below 78% are dropped.
#[must_use]
pub fn trace(img: &Image, cfg: &SkyConfig) -> Vec<Pixel> {
    let raw: Vec<Option<usize>> = (0..img.width).map(|x| ridge_row(img, x, cfg.rock_run)).collect();
    let cleaned = fill_median(&despike(&raw, 70, 50), 8);
    let h = img.height as f64;
    cleaned
        .iter()
        .enumerate()
        .filter_map(|(x, y)| {
            let y = (*y)?;
            let share = y as f64 / h;
            (0.15 < share && share < 0.78).then_some(Pixel { x: x as f64 + 0.5, y: y as f64 + 0.5 })
        })
        .collect()
}

/// About `target` points, evenly spaced along the skyline.
#[must_use]
pub fn decimate(skyline: &[Pixel], target: usize) -> Vec<Pixel> {
    let n = skyline.len().max(1);
    let step = 1.max(n.div_ceil(target.max(1)));
    skyline.iter().step_by(step).copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blue_sky_over_brown_ridge() {
        let (w, h) = (40, 400);
        let sky = 0xff_40_80_e0_u32;
        let rock = 0xff_70_50_30_u32;
        let mut argb = vec![sky; w * h];
        for x in 0..w {
            let top = 120 + x;
            for y in top..h {
                argb[y * w + x] = rock;
            }
        }
        let img = Image { width: w, height: h, argb };
        let line = trace(&img, &SkyConfig::default());
        assert_eq!(line.len(), w);
        // The 8-wide median smooths a ramp's ends toward the middle of their window.
        assert_eq!(line[0].y, 124.5);
        assert_eq!(line[10].y, 130.5);
    }

    #[test]
    fn decimate_spacing() {
        let pts: Vec<Pixel> = (0..1000).map(|x| Pixel { x: f64::from(x), y: 0.0 }).collect();
        let d = decimate(&pts, 160);
        assert_eq!(d.len(), 143, "step ceil(1000/160) = 7");
        assert_eq!(d[1].x, 7.0);
    }
}
