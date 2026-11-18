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
    (
        i64::from((p >> 16) & 0xff),
        i64::from((p >> 8) & 0xff),
        i64::from(p & 0xff),
    )
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
    let sat = if maxv == 0 {
        0.0
    } else {
        (maxv - minv) as f64 / maxv as f64
    };
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
                    if (0..n).contains(&xx) {
                        rows[xx as usize]
                    } else {
                        None
                    }
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
            (Some(y), Some(w)) => {
                if (y as i64) < w as i64 - jump as i64 {
                    None
                } else {
                    Some(y)
                }
            }
        })
        .collect()
}

/// Ridge rows per column after removing spikes and filling gaps.
fn ridge_rows(img: &Image, rock_run: usize) -> Vec<Option<usize>> {
    let raw: Vec<Option<usize>> = (0..img.width)
        .map(|x| ridge_row(img, x, rock_run))
        .collect();
    fill_median(&despike(&raw, 70, 50), 8)
}

/// Skyline pixels in image coordinates. Columns whose ridge is in the top 15% of the frame or
/// below 78% are dropped.
#[must_use]
pub fn trace(img: &Image, cfg: &SkyConfig) -> Vec<Pixel> {
    let h = img.height as f64;
    ridge_rows(img, cfg.rock_run)
        .iter()
        .enumerate()
        .filter_map(|(x, y)| {
            let y = (*y)?;
            let share = y as f64 / h;
            (0.15 < share && share < 0.78).then_some(Pixel {
                x: x as f64 + 0.5,
                y: y as f64 + 0.5,
            })
        })
        .collect()
}

/// A box in image pixels: columns `x0..x1`, rows `y0..y1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub x0: usize,
    pub y0: usize,
    pub x1: usize,
    pub y1: usize,
}

impl Region {
    /// The box clamped to the image, or `None` when less than 16 × 16 pixels of it remains.
    #[must_use]
    pub fn within(self, img: &Image) -> Option<Self> {
        let r = Self {
            x0: self.x0.min(img.width),
            y0: self.y0.min(img.height),
            x1: self.x1.min(img.width),
            y1: self.y1.min(img.height),
        };
        (r.x1 >= r.x0 + 16 && r.y1 >= r.y0 + 16).then_some(r)
    }
}

/// How much blueness counts against brightness in `skyness`. Hazy or sunlit slopes can be
/// nearly as bright as the sky behind them but are clearly less blue, so blueness carries more
/// weight than brightness.
const BLUE_WEIGHT: f64 = 1.0;

/// How sky-like a pixel is: brightness plus weighted blueness (blue minus red). Blue sky scores
/// highest; land, hazy blue-grey rock and green or yellow trees score lower, so the ridge is a
/// step down in this value. A slow fade of the sky towards the horizon is not a step.
#[inline]
fn skyness(p: u32) -> f64 {
    let (r, g, b) = channels(p);
    let (r, g, b) = (r as f64, g as f64, b as f64);
    0.299 * r + 0.587 * g + 0.114 * b + BLUE_WEIGHT * (b - r)
}

/// Smallest step down in `skyness` that counts as a ridge: above the sky's own fade and most
/// cloud edges, below the step from sky to hazy rock. Tuned on a hazy evening photo over the
/// Wasatch, where a relative bar passed over sunlit slopes for the trees beneath them.
const MIN_STEP: f64 = 30.0;

/// The ridge row in one column of a box: the topmost clear step down from sky to land. Each row
/// is scored by the mean skyness of up to `k` rows above it minus the mean of up to `k` rows
/// below (fewer near the box's edges, so a ridge just under the top still counts). The first row
/// whose step reaches `MIN_STEP` wins (then the peak of that step), however strong the edges
/// below it are, so a hazy slope above dark trees is not passed over for the trees.
fn step_row(img: &Image, x: usize, r: Region) -> Option<usize> {
    let h = r.y1 - r.y0;
    let k = (h / 12).max(3);
    let edge = 2;
    if h < 2 * edge + 1 {
        return None;
    }
    let mut prefix = vec![0.0; h + 1];
    for y in 0..h {
        prefix[y + 1] = prefix[y] + skyness(img.argb[(r.y0 + y) * img.width + x]);
    }
    let mean = |a: usize, b: usize| (prefix[b] - prefix[a]) / (b - a) as f64;
    let step = |y: usize| mean(y.saturating_sub(k), y) - mean(y, (y + k).min(h));
    let rows = edge..=h - edge;
    let first = rows.clone().find(|&y| step(y) >= MIN_STEP)?;
    let row = (first..=(first + k).min(h - edge)).max_by(|&a, &b| step(a).total_cmp(&step(b)))?;
    // A "ridge" in the bottom 5% of the box is whatever the box's lower edge cut through, such
    // as tree tops; the column has no ridge the tracer can see.
    (row < h - (h / 20).max(edge)).then_some(row)
}

/// The skyline inside a box someone drew around the mountains, in whole-image coordinates.
/// Inside a box the person has already shown where sky and ridge are, so instead of the
/// whole-photo colour rules (which need clearly blue sky) each column takes the topmost strong
/// step from sky to land. That works for pale, cloudy or evening skies over hazy blue-grey
/// mountains. Trees, poles and clouds outside the box are simply not seen.
#[must_use]
pub fn trace_region(img: &Image, region: Region) -> Vec<Pixel> {
    let Some(r) = region.within(img) else {
        return Vec::new();
    };
    let raw: Vec<Option<usize>> = (r.x0..r.x1).map(|x| step_row(img, x, r)).collect();
    fill_median(&despike(&raw, 70, 50), 8)
        .iter()
        .enumerate()
        .filter_map(|(x, y)| {
            let y = (*y)?;
            Some(Pixel {
                x: (r.x0 + x) as f64 + 0.5,
                y: (r.y0 + y) as f64 + 0.5,
            })
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
        let img = Image {
            width: w,
            height: h,
            argb,
        };
        let line = trace(&img, &SkyConfig::default());
        assert_eq!(line.len(), w);
        // The 8-wide median smooths a ramp's ends toward the middle of their window.
        assert_eq!(line[0].y, 124.5);
        assert_eq!(line[10].y, 130.5);
    }

    #[test]
    fn a_box_sees_the_ridge_and_not_the_tree_beside_it() {
        let (w, h) = (300, 600);
        let sky = 0xff_40_80_e0_u32;
        let rock = 0xff_70_50_30_u32;
        let tree = 0xff_20_60_20_u32;
        let mut argb = vec![sky; w * h];
        for x in 0..w {
            // A flat ridge at row 300, and a tree to row 120 over the first 60 columns.
            let top = if x < 60 { 120 } else { 300 };
            for y in top..h {
                argb[y * w + x] = if x < 60 && y < 300 { tree } else { rock };
            }
        }
        let img = Image {
            width: w,
            height: h,
            argb,
        };
        let full = trace(&img, &SkyConfig::default());
        assert!(
            full.iter().any(|p| p.x < 50.0 && p.y < 200.0),
            "the full trace rides the tree"
        );
        // A box from just above the ridge, right of the tree: the ridge sits 2% down the box,
        // which the full-frame band would drop.
        let boxed = trace_region(
            &img,
            Region {
                x0: 80,
                y0: 296,
                x1: 300,
                y1: 500,
            },
        );
        assert_eq!(boxed.len(), 220, "one point per column of the box");
        assert!(boxed.iter().all(|p| (300.0..=301.0).contains(&p.y)));
        assert_eq!(boxed[0].x, 80.5, "whole-image coordinates");
        assert!(
            Region {
                x0: 0,
                y0: 0,
                x1: 10,
                y1: 400
            }
            .within(&img)
            .is_none(),
            "too narrow"
        );
    }

    #[test]
    fn decimate_spacing() {
        let pts: Vec<Pixel> = (0..1000)
            .map(|x| Pixel {
                x: f64::from(x),
                y: 0.0,
            })
            .collect();
        let d = decimate(&pts, 160);
        assert_eq!(d.len(), 143, "step ceil(1000/160) = 7");
        assert_eq!(d[1].x, 7.0);
    }
}
