//! A whole search: screen and rank each stored field, then refine around the best places with
//! horizons walked fresh on finer grids. The caller supplies a stage timer, so this module never
//! reads a clock.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::Config;
use crate::camera::{Lens, Photo, lenses};
use crate::dem::Dem;
use crate::field::{self, Field};
use crate::score::{Place, Yaws, surface, yaws_around};
use crate::screen::{ScreenRow, Template, screen, templates};

/// The stages of a search, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Stage {
    Prepare,
    Coarse,
    Refine(usize),
}

/// Told when each stage ends; the CLI times them with a wall clock.
pub trait Timer {
    fn mark(&mut self, stage: Stage);
}

/// A timer that records nothing.
pub struct NoTimer;

impl Timer for NoTimer {
    fn mark(&mut self, _: Stage) {}
}

/// Everything a search produces.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Found {
    /// Every place given the full search, best first.
    pub rows: Vec<Place>,
    /// The screen of the stored fields, best first.
    pub screen: Vec<ScreenRow>,
    /// Places scored per stage: stored fields, then each refinement.
    pub counts: Vec<usize>,
}

/// The photograph's lenses and templates, built once per search.
pub struct Prepared {
    pub lenses: Vec<Lens>,
    pub templates: Vec<Template>,
}

impl Prepared {
    #[must_use]
    pub fn new(photo: &Photo, cfg: &Config) -> Self {
        let s = &cfg.score;
        Self {
            lenses: lenses(photo, &s.fovs, &s.rolls, s.pitch_low, s.pitch_high),
            templates: templates(photo, s, &cfg.screen),
        }
    }
}

/// The best places of one field in two passes: the screen over every place, then the full
/// search over the best `max(keep_min, keep_share · n)`, with yaw only near where the screen
/// pointed. Returns the full rows and the screen.
#[must_use]
pub fn rank(field: &Field, dem: &Dem<'_>, prep: &Prepared, cfg: &Config) -> (Vec<Place>, Vec<ScreenRow>) {
    let screened = screen(field, dem, &prep.templates, &cfg.score, &cfg.screen);
    let keep = cfg.screen.keep_min.max((cfg.screen.keep_share * field.len() as f64) as usize);
    let chosen = &screened[..keep.min(screened.len())];
    let part = field.subset(&chosen.iter().map(|r| r.index).collect::<Vec<_>>());
    let yaws: Vec<Vec<u16>> = chosen.iter().map(|r| yaws_around(r.yaw, cfg.screen.reach)).collect();
    let rows = surface(&part, dem, &prep.lenses, Yaws::PerPlace(&yaws), &cfg.score);
    (rows, screened)
}

/// Places within `reach_m` of each of the first `top` rows on a `step_m` grid, first occurrence
/// kept.
fn neighbourhood(rows: &[Place], top: usize, step_m: f64, reach_m: f64) -> Vec<(f64, f64)> {
    let mut seen = BTreeSet::new();
    rows.iter()
        .take(top)
        .flat_map(|r| field::around(r.lat, r.lon, step_m, reach_m))
        .filter(|&(lat, lon)| seen.insert((lat.to_bits(), lon.to_bits())))
        .collect()
}

fn by_loss(rows: &mut [Place]) {
    rows.sort_by(|a, b| a.loss.total_cmp(&b.loss));
}

/// Where `photo` was taken, searching `fields` and refining around the best places.
pub fn search(dem: &Dem<'_>, fields: &[Field], photo: &Photo, cfg: &Config, timer: &mut impl Timer) -> Found {
    let prep = Prepared::new(photo, cfg);
    timer.mark(Stage::Prepare);
    let mut coarse = Vec::new();
    let mut screened = Vec::new();
    for f in fields {
        let (rows, scr) = rank(f, dem, &prep, cfg);
        coarse.extend(rows);
        screened.extend(scr);
    }
    by_loss(&mut coarse);
    screened.sort_by(|a, b| a.loss.total_cmp(&b.loss));
    timer.mark(Stage::Coarse);
    let mut counts = vec![coarse.len()];
    let mut all = coarse.clone();
    let mut previous = coarse;
    for (pass, &(top, step_m, reach_m)) in cfg.refine.passes.iter().enumerate() {
        let places = neighbourhood(&previous, top, step_m, reach_m);
        let fresh = field::gather(dem, &cfg.horizon, &places);
        let (rows, _) = rank(&fresh, dem, &prep, cfg);
        counts.push(rows.len());
        all.extend_from_slice(&rows);
        previous = rows;
        timer.mark(Stage::Refine(pass));
    }
    by_loss(&mut all);
    Found { rows: all, screen: screened, counts }
}

/// Every place within `reach_m` of `lat`, `lon`, on a `step_m` grid, ranked: does the ground
/// near a known camera fit, and by how much does the winner beat it.
#[must_use]
pub fn around(dem: &Dem<'_>, photo: &Photo, cfg: &Config, lat: f64, lon: f64, step_m: f64, reach_m: f64) -> Vec<Place> {
    let prep = Prepared::new(photo, cfg);
    let fresh = field::gather(dem, &cfg.horizon, &field::around(lat, lon, step_m, reach_m));
    rank(&fresh, dem, &prep, cfg).0
}
