//! The skyline from a sky-segmentation map: the one continuous line across the photo that best
//! splits sky above from ground below.
//!
//! The colour rules in [`crate::sky`] decide sky pixel by pixel, so white snow reads as cloud,
//! a dark cloud reads as rock, and a pole or a wire breaks the line for a few columns. Here a
//! trained network (U²-Net, run by `sight-io`) says how sky-like every part of the photo is, and
//! dynamic programming finds the cheapest path from the left edge to the right (Lie et al.'s
//! shortest-path skyline, as in Ahmad et al. 2021): each column pays for sky it puts below the
//! line and ground it puts above, and each step pays for jumping rows, so the line follows the
//! ridge instead of hopping onto a cloud or a roof for a column or two.

use crate::camera::Pixel;
use crate::sky::{Image, MIN_STEP, Region, skyness};

/// How sky-like each part of a photo is, 0 (ground) to 1 (sky), on a `width × height` grid
/// stretched over the whole frame.
#[derive(Clone, Debug, PartialEq)]
pub struct SkyMap {
    pub width: usize,
    pub height: usize,
    pub p: Vec<f32>,
}

impl SkyMap {
    /// The map at a point given as fractions of the frame, bilinear between grid cells.
    #[must_use]
    pub fn at(&self, u: f64, v: f64) -> f64 {
        if self.width == 0 || self.height == 0 || self.p.len() < self.width * self.height {
            return 0.5;
        }
        let x = (u * self.width as f64 - 0.5).clamp(0.0, (self.width - 1) as f64);
        let y = (v * self.height as f64 - 0.5).clamp(0.0, (self.height - 1) as f64);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.width - 1), (y0 + 1).min(self.height - 1));
        let (fx, fy) = (x - x0 as f64, y - y0 as f64);
        let g = |cx: usize, cy: usize| f64::from(self.p[cy * self.width + cx]);
        let top = g(x0, y0) * (1.0 - fx) + g(x1, y0) * fx;
        let bottom = g(x0, y1) * (1.0 - fx) + g(x1, y1) * fx;
        top * (1.0 - fy) + bottom * fy
    }
}

/// Columns the path is solved on, at most: finer than the 160 the search scores, and a few
/// pixels apart even on a 4000-pixel photo.
const MAX_COLUMNS: usize = 1024;
/// Rows the path is solved on, at most. A 320-cell map has no finer detail; the line is snapped
/// to the photo's own pixels afterwards.
const MAX_ROWS: usize = 640;
/// Cost of jumping the full height of the box between neighbouring columns, against a column
/// cost of at most 1. Small enough that a cliff or a peak's flank is followed, large enough that
/// a cloud or a pole a few columns wide is not worth leaving the ridge for.
const JUMP: f64 = 6.0;
/// A column keeps its point only if the map is clearly sky just above it and clearly ground just
/// below. Where the network is unsure (haze, a bright snowfield against pale cloud) the column
/// is left out rather than guessed.
const SURE: f64 = 0.15;
/// Where the map leans sky above the line (at least this much) without being sure, the column is
/// kept if the photo itself steps down from sky to ground there by twice the box tracer's
/// `MIN_STEP`: grey cloud over a dark summit is half-sure in the map but a hard edge in the photo,
/// while the soft base of a cloud resting on the slope is neither.
const LEANS: f64 = 0.5;
/// What ground hanging from the top of the frame costs, per row, against 1 for ground anywhere
/// else above the line: branches or an awning in front of the sky, with sky between them and the
/// ridge. Sky inside that hanging stretch still costs in full, so it cannot swallow the real sky
/// above a ridge to put the line under a lake's reflection.
const OVERHANG: f64 = 0.1;

/// The whole photo as a region.
#[must_use]
pub fn whole(img: &Image) -> Region {
    Region {
        x0: 0,
        y0: 0,
        x1: img.width,
        y1: img.height,
    }
}

/// The skyline inside `region` (the whole photo when it covers the frame), in image pixels.
#[must_use]
pub fn trace_skymap(img: &Image, map: &SkyMap, region: Region) -> Vec<Pixel> {
    let Some(r) = region.within(img) else {
        return Vec::new();
    };
    let (bw, bh) = (r.x1 - r.x0, r.y1 - r.y0);
    let cols = bw.min(MAX_COLUMNS);
    let rows = bh.min(MAX_ROWS);
    let (w, h) = (img.width as f64, img.height as f64);
    let col_x = |c: usize| r.x0 as f64 + (c as f64 + 0.5) * bw as f64 / cols as f64;
    let row_y = |k: usize| r.y0 as f64 + k as f64 * bh as f64 / rows as f64;

    // cost[c][k]: the line in column c sits between rows k-1 and k (k = 0: everything ground,
    // k = rows: everything sky). It pays for ground above it and sky below it, except that rows
    // 0..j above it may be taken as an overhang, where ground costs OVERHANG and sky costs 1.
    let mut cost = vec![0.0_f64; cols * (rows + 1)];
    let mut probs = vec![0.0_f64; cols * rows];
    for c in 0..cols {
        let u = col_x(c) / w;
        let p: Vec<f64> = (0..rows)
            .map(|k| map.at(u, (row_y(k) + 0.5 * bh as f64 / rows as f64) / h))
            .collect();
        // ground[k]: ground in rows 0..k; sky[k]: sky in rows 0..k.
        let mut ground = vec![0.0_f64; rows + 1];
        let mut sky = vec![0.0_f64; rows + 1];
        for k in 0..rows {
            ground[k + 1] = ground[k] + 1.0 - p[k];
            sky[k + 1] = sky[k] + p[k];
        }
        // Cost(k) = min over j ≤ k of sky[j] + OVERHANG·ground[j] + (ground[k] − ground[j])
        //         + (sky[rows] − sky[k]); the j terms are a running minimum.
        let mut hang = f64::INFINITY;
        for k in 0..=rows {
            hang = hang.min(sky[k] - (1.0 - OVERHANG) * ground[k]);
            cost[c * (rows + 1) + k] = (hang + ground[k] + sky[rows] - sky[k]) / rows as f64;
        }
        probs[c * rows..(c + 1) * rows].copy_from_slice(&p);
    }

    // Dynamic programming left to right. A jump of d rows costs JUMP · d / rows; the minimum over
    // the previous column is an L1 distance transform, two passes per column.
    let step = JUMP / rows as f64;
    let n = rows + 1;
    let mut total = cost[..n].to_vec();
    let mut back = vec![0_u32; cols * n];
    for c in 1..cols {
        let mut best = total.clone();
        let mut from: Vec<u32> = (0..n as u32).collect();
        for k in 1..n {
            if best[k - 1] + step < best[k] {
                best[k] = best[k - 1] + step;
                from[k] = from[k - 1];
            }
        }
        for k in (0..n - 1).rev() {
            if best[k + 1] + step < best[k] {
                best[k] = best[k + 1] + step;
                from[k] = from[k + 1];
            }
        }
        for k in 0..n {
            total[k] = best[k] + cost[c * n + k];
            back[c * n + k] = from[k];
        }
    }
    let mut k = (0..n)
        .min_by(|&a, &b| total[a].total_cmp(&total[b]))
        .unwrap_or(0);
    let mut line = vec![0_usize; cols];
    for c in (0..cols).rev() {
        line[c] = k;
        k = back[c * n + k] as usize;
    }

    // Keep the columns where the map is sure, and snap each to the photo's own sharpest step
    // from sky to ground within a grid cell of the solved row.
    let band = (rows / 40).max(2);
    let cell = (bh as f64 / rows as f64).ceil() as usize + 1;
    line.iter()
        .enumerate()
        .filter_map(|(c, &k)| {
            if k == 0 || k >= rows {
                return None;
            }
            let p = &probs[c * rows..(c + 1) * rows];
            let above = p[k.saturating_sub(band)..k].iter().sum::<f64>()
                / (k - k.saturating_sub(band)) as f64;
            let end = (k + band).min(rows);
            let below = p[k..end].iter().sum::<f64>() / (end - k) as f64;
            if above < LEANS || below > SURE + 0.35 {
                return None;
            }
            let x = col_x(c).floor() as usize;
            let (y, step) = snap(
                img,
                x.min(img.width - 1),
                row_y(k).round() as usize,
                cell,
                r,
            );
            if above < 1.0 - SURE && step < 2.0 * MIN_STEP {
                return None;
            }
            Some(Pixel {
                x: x as f64 + 0.5,
                y: y as f64 + 0.5,
            })
        })
        .collect()
}

/// Rise between neighbouring points, as a share of the photo height, that counts as a wall: a
/// building side, not a slope. Cliffs this sheer are rare and short.
const WALL: f64 = 0.02;
/// A stretch between walls this short (share of the traced width), and flatter than the walls
/// it stands on, is a building, a tower or a tree crown in front of the ridge.
const STRUCTURE_SHARE: f64 = 0.2;

/// The line without stretches that stand on walls above their neighbours: roofs and towers that
/// rise above the ridge. They are dropped, not filled; the search scores the rest.
#[must_use]
pub fn drop_structures(line: &[Pixel], height: usize) -> Vec<Pixel> {
    if line.len() < 3 {
        return line.to_vec();
    }
    let wall = (WALL * height as f64).max(6.0);
    let mut cuts = vec![0];
    for i in 1..line.len() {
        if (line[i].y - line[i - 1].y).abs() >= wall {
            cuts.push(i);
        }
    }
    cuts.push(line.len());
    let segments: Vec<(usize, usize)> = cuts.windows(2).map(|w| (w[0], w[1])).collect();
    let span = line.len() as f64;
    let mut keep = vec![true; line.len()];
    for (j, &(a, b)) in segments.iter().enumerate() {
        if segments.len() == 1 || (b - a) as f64 > STRUCTURE_SHARE * span {
            continue;
        }
        let top = line[a..b].iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let bottom = line[a..b]
            .iter()
            .map(|p| p.y)
            .fold(f64::NEG_INFINITY, f64::max);
        // How far it stands above each neighbour (positive: higher in the frame).
        let left = (j > 0).then(|| line[a - 1].y - line[a].y);
        let right = (j + 1 < segments.len()).then(|| line[b].y - line[b - 1].y);
        let rises: Vec<f64> = [left, right].into_iter().flatten().collect();
        let rise = rises.iter().copied().fold(f64::INFINITY, f64::min);
        if rise >= wall && bottom - top < rise {
            keep[a..b].iter_mut().for_each(|k| *k = false);
        }
    }
    line.iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect()
}

/// The row near `y` (within `reach`) where the photo drops most from sky-like to ground-like,
/// comparing a few pixels above with a few below, and that drop; `y` and 0 when there is no
/// room to look.
fn snap(img: &Image, x: usize, y: usize, reach: usize, r: Region) -> (usize, f64) {
    let k = 3;
    let lo = y.saturating_sub(reach).max(r.y0 + k);
    let hi = (y + reach).min(r.y1.saturating_sub(k)).min(img.height - k);
    if lo >= hi {
        return (y, 0.0);
    }
    let at = |row: usize| skyness(img.argb[row * img.width + x]);
    let mean = |a: usize, b: usize| (a..b).map(at).sum::<f64>() / (b - a) as f64;
    (lo..hi)
        .map(|row| (row, mean(row - k, row) - mean(row, row + k)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap_or((y, 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A photo whose map says sky above a sloping ridge, with a "cloud" of ground-like map
    /// values floating in the sky and a "lake" of sky-like values below the ridge.
    #[test]
    fn follows_the_ridge_past_a_cloud_and_a_lake() {
        let (w, h) = (300_usize, 200_usize);
        let ridge = |x: usize| 80 + x / 5;
        let argb = (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                if y < ridge(x) {
                    0xff_60_90_e0
                } else {
                    0xff_50_40_30
                }
            })
            .collect();
        let img = Image {
            width: w,
            height: h,
            argb,
        };
        let (mw, mh) = (60, 40);
        let p = (0..mw * mh)
            .map(|i| {
                let (cx, cy) = (i % mw, i / mw);
                let (x, y) = (cx * w / mw, cy * h / mh);
                let cloud = (100..140).contains(&x) && (20..40).contains(&y);
                let lake = y > 180;
                if (y < ridge(x) && !cloud) || lake {
                    1.0
                } else {
                    0.0
                }
            })
            .collect();
        let map = SkyMap {
            width: mw,
            height: mh,
            p,
        };
        let line = trace_skymap(
            &img,
            &map,
            Region {
                x0: 0,
                y0: 0,
                x1: w,
                y1: h,
            },
        );
        assert!(
            line.len() > w * 8 / 10,
            "only {} of {w} columns",
            line.len()
        );
        for pt in &line {
            let want = ridge(pt.x as usize) as f64;
            assert!(
                (pt.y - want).abs() <= 6.0,
                "x {} at {} not {want}",
                pt.x,
                pt.y
            );
        }
    }
}
