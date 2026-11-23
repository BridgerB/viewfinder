//! `sight`: build horizon fields from a DEM, locate photographs, and measure the search.

#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::too_many_arguments,
    clippy::float_cmp,
    clippy::cast_possible_wrap,
    reason = "a CLI: table formatting, index arithmetic, and the golden check compares floats exactly on purpose"
)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::Serialize;
use sight_core::Config;
use sight_core::camera::{Photo, Pose, outline};
use sight_core::field::{self, Field};
use sight_core::horizon::{self, AZIMUTHS};
use sight_core::java::JavaRandom;
use sight_core::score::Place;
use sight_core::search::{self, Found, Stage, Timer};
use sight_core::sky;
use sight_io::LoadedDem;

mod serve;

#[derive(Parser)]
#[command(name = "sight", about = "Where was this mountain photograph taken?")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Bounds of the grid and the ground near each corner.
    Info { grid: PathBuf },
    /// A horizon field on a STEP-meter lattice over the grid, or over SOUTH NORTH WEST EAST.
    Build {
        grid: PathBuf,
        out: PathBuf,
        step: f64,
        #[arg(num_args = 4)]
        bbox: Vec<f64>,
    },
    /// A horizon field on a STEP-meter lattice, only where the ground within RADIUS spans RELIEF.
    BuildRough {
        grid: PathBuf,
        out: PathBuf,
        step: f64,
        radius: f64,
        relief: f64,
        /// Only places inside SOUTH NORTH WEST EAST; the whole grid when left out.
        #[arg(num_args = 4)]
        bbox: Vec<f64>,
    },
    /// The same places as an existing field, walked again on GRID (for example a larger mosaic,
    /// so rays that used to leave the tile now see the ground beyond it).
    Rewalk {
        grid: PathBuf,
        field: PathBuf,
        out: PathBuf,
    },
    /// Where a photograph was taken: writes the heatmap and the winning camera's horizon.
    Search {
        grid: PathBuf,
        /// Comma-separated horizon fields.
        fields: String,
        image: PathBuf,
        surface: PathBuf,
        matched: PathBuf,
        #[arg(long)]
        overlay: Option<PathBuf>,
        /// A skyline file of [x, y] pairs instead of tracing the photograph.
        #[arg(long)]
        skyline: Option<PathBuf>,
        /// The lens width in degrees, when the camera's focal length is known.
        #[arg(long)]
        fov: Option<f64>,
    },
    /// Every place within REACH m of LAT LON, STEP m apart, scored against the photograph.
    Around {
        grid: PathBuf,
        image: PathBuf,
        lat: f64,
        lon: f64,
        #[arg(long)]
        skyline: Option<PathBuf>,
        #[arg(long)]
        fov: Option<f64>,
        #[arg(long, default_value_t = 125.0)]
        step: f64,
        #[arg(long, default_value_t = 1000.0)]
        reach: f64,
    },
    /// The ridge each camera's skyline sits on, for lines of `id lat lon yaw fov`.
    Ridges {
        grid: PathBuf,
        cameras: PathBuf,
        out: PathBuf,
    },
    /// The traced skyline, as fractions of the frame.
    Trace { image: PathBuf },
    /// The skyline traced inside a box (image pixels), optionally drawn onto a copy of the photo.
    TraceBox {
        image: PathBuf,
        x0: usize,
        y0: usize,
        x1: usize,
        y1: usize,
        #[arg(long)]
        overlay: Option<PathBuf>,
    },
    /// The skyline from the sky-segmentation network (MODEL is skyseg.onnx), optionally drawn
    /// onto a copy of the photo, with the network's time.
    SkyTrace {
        image: PathBuf,
        model: PathBuf,
        #[arg(long)]
        overlay: Option<PathBuf>,
        /// A hand-traced skyline to draw beside the trace, for comparison.
        #[arg(long)]
        guide: Option<PathBuf>,
    },
    /// Every known-location photograph searched, timed and checked against its truth.
    Bench {
        grid: PathBuf,
        fields: String,
        cases: PathBuf,
        #[arg(long)]
        quick: bool,
        #[arg(long, default_value = "data/bench/runs")]
        runs: PathBuf,
        /// Trace with the sky-segmentation network (skyseg.onnx) instead of the colour rules.
        #[arg(long)]
        sky: Option<PathBuf>,
        /// Ignore the hand-traced skylines and trace every photo, to test the tracer.
        #[arg(long)]
        no_guides: bool,
    },
    /// A fixed sample of PLACES scored REPS times: the workload for profilers.
    Profile {
        grid: PathBuf,
        fields: String,
        image: PathBuf,
        #[arg(long)]
        skyline: Option<PathBuf>,
        #[arg(long, default_value_t = 500)]
        places: usize,
        #[arg(long, default_value_t = 3)]
        reps: usize,
    },
    /// Value-by-value comparison of two horizon fields.
    CompareFields { a: PathBuf, b: PathBuf },
    /// The full search at the golden sample of places, compared with a recorded answer.
    Golden {
        grid: PathBuf,
        field: PathBuf,
        image: PathBuf,
        answer: PathBuf,
        #[arg(long)]
        skyline: Option<PathBuf>,
    },
    /// Column-by-column comparison of the traced skyline with a skyline file.
    CompareTrace { image: PathBuf, skyline: PathBuf },
    /// The locator as a local HTTP API for the site's upload page: POST /trace, GET /image/ID,
    /// POST /locate. Keeps the grid and fields in memory.
    Serve {
        grid: PathBuf,
        /// Comma-separated horizon fields.
        fields: String,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 4373)]
        port: u16,
        /// Trace with the sky-segmentation network (skyseg.onnx) instead of the colour rules.
        #[arg(long)]
        sky: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Info { grid } => info(&grid),
        Cmd::Rewalk { grid, field, out } => rewalk(&grid, &field, &out),
        Cmd::Build {
            grid,
            out,
            step,
            bbox,
        } => build(&grid, &out, step, &bbox),
        Cmd::BuildRough {
            grid,
            out,
            step,
            radius,
            relief,
            bbox,
        } => build_rough(&grid, &out, step, radius, relief, &bbox),
        Cmd::Search {
            grid,
            fields,
            image,
            surface,
            matched,
            overlay,
            skyline,
            fov,
        } => search_cmd(
            &grid,
            &fields,
            &image,
            &surface,
            &matched,
            overlay.as_deref(),
            skyline.as_deref(),
            fov,
        ),
        Cmd::Around {
            grid,
            image,
            lat,
            lon,
            skyline,
            fov,
            step,
            reach,
        } => around(
            &grid,
            &image,
            lat,
            lon,
            skyline.as_deref(),
            fov,
            step,
            reach,
        ),
        Cmd::Ridges { grid, cameras, out } => ridges(&grid, &cameras, &out),
        Cmd::Trace { image } => trace(&image),
        Cmd::TraceBox {
            image,
            x0,
            y0,
            x1,
            y1,
            overlay,
        } => trace_box(&image, sky::Region { x0, y0, x1, y1 }, overlay.as_deref()),
        Cmd::SkyTrace {
            image,
            model,
            overlay,
            guide,
        } => sky_trace(&image, &model, overlay.as_deref(), guide.as_deref()),
        Cmd::Bench {
            grid,
            fields,
            cases,
            quick,
            runs,
            sky,
            no_guides,
        } => bench(
            &grid,
            &fields,
            &cases,
            quick,
            &runs,
            sky.as_deref(),
            no_guides,
        ),
        Cmd::Profile {
            grid,
            fields,
            image,
            skyline,
            places,
            reps,
        } => profile(&grid, &fields, &image, skyline.as_deref(), places, reps),
        Cmd::CompareFields { a, b } => compare_fields(&a, &b),
        Cmd::Golden {
            grid,
            field,
            image,
            answer,
            skyline,
        } => golden(&grid, &field, &image, &answer, skyline.as_deref()),
        Cmd::CompareTrace { image, skyline } => compare_trace(&image, &skyline),
        Cmd::Serve {
            grid,
            fields,
            host,
            port,
            sky,
        } => serve::serve(&grid, &fields, &host, port, sky.as_deref()),
    }
}

fn config(fov: Option<f64>) -> Config {
    match fov {
        Some(f) => Config::default().with_known_fov(f),
        None => Config::default(),
    }
}

/// The photograph and its skyline: traced, or read from a file, then decimated.
fn photo(
    image: &Path,
    skyline: Option<&Path>,
    cfg: &Config,
) -> Result<(Photo, Vec<sight_core::camera::Pixel>)> {
    photo_with(image, skyline, cfg, None)
}

/// As `photo`, tracing with the sky-segmentation network when one is given.
fn photo_with(
    image: &Path,
    skyline: Option<&Path>,
    cfg: &Config,
    net: Option<&sight_io::skyseg::SkyModel>,
) -> Result<(Photo, Vec<sight_core::camera::Pixel>)> {
    let img = sight_io::read_image(image)?;
    let line = match (skyline, net) {
        (Some(p), _) => sight_io::read_skyline(p)?,
        (None, Some(net)) => traced_with_sky(&img, &net.sky(&img)?, None, &cfg.sky),
        (None, None) => sky::trace_best(&img, &cfg.sky),
    };
    let sampled = sky::decimate(&line, cfg.sky.columns);
    Ok((
        Photo {
            width: img.width as u32,
            height: img.height as u32,
            skyline: sampled,
        },
        line,
    ))
}

/// The skyline from a sky map, inside `region` or over the whole photo; the colour rules when
/// the map finds almost nothing (a photo the network cannot read).
pub(crate) fn traced_with_sky(
    img: &sky::Image,
    map: &sight_core::skymap::SkyMap,
    region: Option<sky::Region>,
    cfg: &sight_core::config::SkyConfig,
) -> Vec<sight_core::camera::Pixel> {
    let area = region.unwrap_or_else(|| sight_core::skymap::whole(img));
    let line = sight_core::skymap::drop_structures(
        &sight_core::skymap::trace_skymap(img, map, area),
        img.height,
    );
    if line.len() >= 24 {
        line
    } else if let Some(r) = region {
        sky::trace_region(img, r)
    } else {
        sky::trace_best(img, cfg)
    }
}

fn info(grid: &Path) -> Result<()> {
    let loaded = LoadedDem::open(grid)?;
    let dem = loaded.dem();
    let (n, w, s, e) = (dem.geo.north, dem.geo.west, dem.south(), dem.east());
    println!(
        "north {n} south {s} west {w} east {e} width {} height {}",
        dem.geo.width, dem.geo.height
    );
    for (name, lat, lon) in [
        ("nw", n - 0.01, w + 0.01),
        ("ne", n - 0.01, e - 0.01),
        ("sw", s + 0.01, w + 0.01),
        ("se", s + 0.01, e - 0.01),
    ] {
        println!("{name} {:?}", dem.elev_at(lat, lon));
    }
    Ok(())
}

fn build(grid: &Path, out: &Path, step: f64, bbox: &[f64]) -> Result<()> {
    let t0 = Instant::now();
    let loaded = LoadedDem::open(grid)?;
    let dem = loaded.dem();
    let (s, n, w, e) = match bbox {
        [s, n, w, e] => (*s, *n, *w, *e),
        _ => (dem.south(), dem.geo.north, dem.geo.west, dem.east()),
    };
    let places = field::lattice(s, n, w, e, step);
    let f = field::gather(&dem, &Config::default().horizon, &places);
    sight_io::write_field(out, &f)?;
    println!(
        "places {} wrote {} to {} in {:.2} s",
        places.len(),
        f.len(),
        out.display(),
        t0.elapsed().as_secs_f64()
    );
    Ok(())
}

fn build_rough(
    grid: &Path,
    out: &Path,
    step: f64,
    radius: f64,
    relief: f64,
    bbox: &[f64],
) -> Result<()> {
    let t0 = Instant::now();
    let loaded = LoadedDem::open(grid)?;
    let dem = loaded.dem();
    let (s, n, w, e) = match bbox {
        [s, n, w, e] => (*s, *n, *w, *e),
        _ => (dem.south(), dem.geo.north, dem.geo.west, dem.east()),
    };
    let all = field::lattice(s, n, w, e, step);
    let rough: Vec<(f64, f64)> = all
        .iter()
        .copied()
        .filter(|&(lat, lon)| field::rugged(&dem, lat, lon, radius, relief))
        .collect();
    let f = field::gather(&dem, &Config::default().horizon, &rough);
    sight_io::write_field(out, &f)?;
    println!(
        "lattice {} rough {} wrote {} to {} in {:.2} s",
        all.len(),
        rough.len(),
        f.len(),
        out.display(),
        t0.elapsed().as_secs_f64()
    );
    Ok(())
}

fn rewalk(grid: &Path, field: &Path, out: &Path) -> Result<()> {
    let t0 = Instant::now();
    let loaded = LoadedDem::open(grid)?;
    let dem = loaded.dem();
    let old = sight_io::read_field(field)?;
    let places: Vec<(f64, f64)> = old
        .lats
        .iter()
        .copied()
        .zip(old.lons.iter().copied())
        .collect();
    let f = field::gather(&dem, &Config::default().horizon, &places);
    sight_io::write_field(out, &f)?;
    println!(
        "places {} wrote {} to {} in {:.2} s",
        places.len(),
        f.len(),
        out.display(),
        t0.elapsed().as_secs_f64()
    );
    Ok(())
}

/// Wall-clock stage times for the CLI.
struct Clock {
    last: Instant,
    stages: Vec<(String, f64)>,
}

impl Clock {
    fn new() -> Self {
        Self {
            last: Instant::now(),
            stages: Vec::new(),
        }
    }

    fn lap(&mut self, name: &str) {
        let now = Instant::now();
        self.stages
            .push((name.to_string(), (now - self.last).as_secs_f64()));
        self.last = now;
    }
}

impl Timer for Clock {
    fn mark(&mut self, stage: Stage) {
        let name = match stage {
            Stage::Prepare => "prepare".to_string(),
            Stage::Coarse => "coarse".to_string(),
            Stage::Refine(i) => format!("refine{i}"),
        };
        self.lap(&name);
    }
}

fn stage_line(stages: &[(String, f64)]) -> String {
    stages
        .iter()
        .map(|(n, s)| format!("{n} {s:.2}"))
        .collect::<Vec<_>>()
        .join("  ")
}

fn search_cmd(
    grid: &Path,
    fields: &str,
    image: &Path,
    surface: &Path,
    matched: &Path,
    overlay: Option<&Path>,
    skyline: Option<&Path>,
    fov: Option<f64>,
) -> Result<()> {
    let mut clock = Clock::new();
    let cfg = config(fov);
    let loaded = LoadedDem::open(grid)?;
    let dem = loaded.dem();
    let fields = sight_io::read_fields(fields)?;
    clock.lap("load");
    let (photo, line) = photo(image, skyline, &cfg)?;
    clock.lap("photo");
    let found = search::search(&dem, &fields, &photo, &cfg, &mut clock);
    let best = *found.rows.first().context("no place scored")?;
    let h =
        horizon::view(&dem, &cfg.horizon, best.lat, best.lon).context("winner has no ground")?;
    let pose = Pose {
        yaw: best.yaw,
        fov: best.fov,
        roll: best.roll,
        pitch: best.pitch,
    };
    let (w, hgt) = (f64::from(photo.width), f64::from(photo.height));
    let drawn = outline(&h.angles, pose, w, hgt);
    sight_io::write_surface(surface, &found.rows)?;
    sight_io::write_match(matched, &best, &drawn, photo.width, photo.height)?;
    if let Some(o) = overlay {
        sight_io::write_overlay(image, o, &line, &drawn)?;
    }
    clock.lap("write");
    println!("{}", stage_line(&clock.stages));
    println!("places {:?}", found.counts);
    for r in found.rows.iter().take(5) {
        println!("{r:?}");
    }
    Ok(())
}

fn around(
    grid: &Path,
    image: &Path,
    lat: f64,
    lon: f64,
    skyline: Option<&Path>,
    fov: Option<f64>,
    step: f64,
    reach: f64,
) -> Result<()> {
    let cfg = config(fov);
    let loaded = LoadedDem::open(grid)?;
    let (photo, _) = photo(image, skyline, &cfg)?;
    for r in search::around(&loaded.dem(), &photo, &cfg, lat, lon, step, reach)
        .iter()
        .take(5)
    {
        println!("{r:?}");
    }
    Ok(())
}

fn ridges(grid: &Path, cameras: &Path, out: &Path) -> Result<()> {
    let loaded = LoadedDem::open(grid)?;
    let dem = loaded.dem();
    let cfg = Config::default();
    let text = std::fs::read_to_string(cameras).with_context(|| cameras.display().to_string())?;
    let mut rows = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let [id, lat, lon, yaw, fov] = parts.as_slice() else {
            continue;
        };
        let (lat, lon, yaw, fov): (f64, f64, f64, f64) =
            (lat.parse()?, lon.parse()?, yaw.parse()?, fov.parse()?);
        let pts = horizon::view(&dem, &cfg.horizon, lat, lon)
            .map(|h| sight_core::geo::ridge(&h.distances, lat, lon, yaw, fov))
            .unwrap_or_default();
        rows.push(((*id).to_string(), pts));
    }
    sight_io::write_ridges(out, &rows)?;
    println!("cameras {} wrote {}", rows.len(), out.display());
    Ok(())
}

fn trace(image: &Path) -> Result<()> {
    let img = sight_io::read_image(image)?;
    let line = sky::trace_best(&img, &Config::default().sky);
    println!(
        "width {} height {} points {}",
        img.width,
        img.height,
        line.len()
    );
    let step = 1.max(line.len() / 36);
    for p in line.iter().step_by(step) {
        println!(
            "{:.4} {:.4}",
            p.x / img.width as f64,
            p.y / img.height as f64
        );
    }
    Ok(())
}

fn sky_trace(
    image: &Path,
    model: &Path,
    overlay: Option<&Path>,
    guide: Option<&Path>,
) -> Result<()> {
    let t0 = Instant::now();
    let net = sight_io::skyseg::SkyModel::load(model)?;
    let loaded = t0.elapsed().as_secs_f64();
    let img = sight_io::read_image(image)?;
    let t1 = Instant::now();
    let map = net.sky(&img)?;
    let ran = t1.elapsed().as_secs_f64();
    let t2 = Instant::now();
    let whole = sky::Region {
        x0: 0,
        y0: 0,
        x1: img.width,
        y1: img.height,
    };
    let line = sight_core::skymap::drop_structures(
        &sight_core::skymap::trace_skymap(&img, &map, whole),
        img.height,
    );
    println!(
        "{}x{} points {} · load {loaded:.2} s, network {ran:.2} s, path {:.3} s",
        img.width,
        img.height,
        line.len(),
        t2.elapsed().as_secs_f64()
    );
    if let Some(out) = overlay {
        let other: Vec<(f64, f64)> = match guide {
            Some(g) => sight_io::read_skyline(g)?
                .iter()
                .map(|p| (p.x, p.y))
                .collect(),
            None => Vec::new(),
        };
        sight_io::write_overlay(image, out, &line, &other)?;
        println!("wrote {}", out.display());
    }
    Ok(())
}

fn trace_box(image: &Path, region: sky::Region, overlay: Option<&Path>) -> Result<()> {
    let img = sight_io::read_image(image)?;
    let line = sky::trace_region(&img, region);
    let ys: Vec<f64> = line.iter().map(|p| p.y).collect();
    let (lo, hi) = ys
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &y| {
            (a.min(y), b.max(y))
        });
    println!(
        "points {} of {} columns, rows {lo:.0}..{hi:.0}",
        line.len(),
        region.x1 - region.x0
    );
    if let Some(out) = overlay {
        sight_io::write_overlay(image, out, &line, &[])?;
        println!("wrote {}", out.display());
    }
    Ok(())
}

fn km_between(a: [f64; 2], b: [f64; 2]) -> f64 {
    let dlat = (b[0] - a[0]) * 111.32;
    let dlon = (b[1] - a[1]) * 111.32 * a[0].to_radians().cos();
    dlat.hypot(dlon)
}

fn first_within(truth: [f64; 2], points: impl Iterator<Item = [f64; 2]>) -> Option<usize> {
    points
        .enumerate()
        .find(|(_, p)| km_between(truth, *p) <= 1.0)
        .map(|(i, _)| i + 1)
}

#[derive(Serialize, serde::Deserialize)]
struct BenchResult {
    id: String,
    error_km: f64,
    baseline_km: f64,
    truth_rank: Option<usize>,
    screen_rank: Option<usize>,
    fov: f64,
    yaw: f64,
    loss: f64,
    seconds: f64,
    stages: Vec<(String, f64)>,
    counts: Vec<usize>,
}

#[derive(Serialize, serde::Deserialize)]
struct BenchRun {
    at: String,
    git: String,
    quick: bool,
    load_seconds: f64,
    cases: Vec<BenchResult>,
}

fn latest_run(dir: &Path) -> Option<BenchRun> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().is_some_and(|x| x == "json")
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("rust-"))
        })
        .collect();
    files.sort();
    serde_json::from_str(&std::fs::read_to_string(files.last()?).ok()?).ok()
}

fn bench(
    grid: &Path,
    fields: &str,
    cases: &Path,
    quick: bool,
    runs: &Path,
    sky: Option<&Path>,
    no_guides: bool,
) -> Result<()> {
    let net = sky.map(sight_io::skyseg::SkyModel::load).transpose()?;
    let before = latest_run(runs);
    let t0 = Instant::now();
    let loaded = LoadedDem::open(grid)?;
    let dem = loaded.dem();
    let fields = sight_io::read_fields(fields)?;
    let load_seconds = t0.elapsed().as_secs_f64();
    let mut results = Vec::new();
    for case in sight_io::read_cases(cases)?
        .into_iter()
        .filter(|c| !quick || c.quick)
    {
        let cfg = config(case.fov);
        let start = Instant::now();
        let mut clock = Clock::new();
        let guide = if no_guides {
            None
        } else {
            case.skyline.as_deref()
        };
        let (photo, _) = photo_with(&case.image, guide, &cfg, net.as_ref())?;
        clock.lap("photo");
        let found: Found = search::search(&dem, &fields, &photo, &cfg, &mut clock);
        let seconds = start.elapsed().as_secs_f64();
        let best: Place = *found.rows.first().context("no place scored")?;
        let r = BenchResult {
            id: case.id.clone(),
            error_km: km_between(case.truth, [best.lat, best.lon]),
            baseline_km: case.baseline_km,
            truth_rank: first_within(case.truth, found.rows.iter().map(|p| [p.lat, p.lon])),
            screen_rank: first_within(case.truth, found.screen.iter().map(|p| [p.lat, p.lon])),
            fov: best.fov,
            yaw: best.yaw,
            loss: best.loss,
            seconds,
            stages: clock.stages,
            counts: found.counts,
        };
        println!(
            "{:<18} {:6.2} km (baseline {:5.2})  rank {:<6} screen rank {:<6} {:6.2} s   {}",
            r.id,
            r.error_km,
            r.baseline_km,
            r.truth_rank.map_or("-".into(), |v| v.to_string()),
            r.screen_rank.map_or("-".into(), |v| v.to_string()),
            r.seconds,
            stage_line(&r.stages)
        );
        results.push(r);
    }
    let git = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let at = format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
    );
    let run = BenchRun {
        at: at.clone(),
        git,
        quick,
        load_seconds,
        cases: results,
    };
    std::fs::create_dir_all(runs)?;
    let out = runs.join(format!("rust-{at}.json"));
    std::fs::write(&out, serde_json::to_string_pretty(&run)?)?;
    println!(
        "\n{:<18} {:>8} {:>8} {:>9} {:>8} {:>8} {:>6}",
        "case", "km", "was km", "baseline", "seconds", "was s", "rank"
    );
    for r in &run.cases {
        let was = before
            .as_ref()
            .and_then(|b| b.cases.iter().find(|c| c.id == r.id));
        println!(
            "{:<18} {:>8.2} {:>8} {:>9.2} {:>8.2} {:>8} {:>6}",
            r.id,
            r.error_km,
            was.map_or("-".into(), |w| format!("{:.2}", w.error_km)),
            r.baseline_km,
            r.seconds,
            was.map_or("-".into(), |w| format!("{:.2}", w.seconds)),
            r.truth_rank.map_or("-".into(), |v| v.to_string())
        );
    }
    let total: f64 = run.cases.iter().map(|c| c.seconds).sum();
    println!(
        "load {load_seconds:.2} s, search total {total:.2} s, wrote {}",
        out.display()
    );
    Ok(())
}

fn profile(
    grid: &Path,
    fields: &str,
    image: &Path,
    skyline: Option<&Path>,
    places: usize,
    reps: usize,
) -> Result<()> {
    let cfg = Config::default();
    let loaded = LoadedDem::open(grid)?;
    let dem = loaded.dem();
    let fields = sight_io::read_fields(fields)?;
    let total: usize = fields.iter().map(Field::len).sum();
    let mut rng = JavaRandom::new(42);
    let parts: Vec<Field> = fields
        .iter()
        .map(|f| {
            let k = ((places as f64 * f.len() as f64) / total as f64).ceil() as usize;
            f.subset(&rng.distinct_sorted(f.len() as i32, k.min(f.len())))
        })
        .collect();
    let (photo, _) = photo(image, skyline, &cfg)?;
    let prep = search::Prepared::new(&photo, &cfg);
    let run = || {
        for part in &parts {
            let _ = search::rank(part, &dem, &prep, &cfg);
        }
    };
    run();
    let times: Vec<f64> = (0..reps)
        .map(|_| {
            let t = Instant::now();
            run();
            t.elapsed().as_secs_f64()
        })
        .collect();
    println!(
        "places {} reps {reps} seconds {:?} threads {}",
        parts.iter().map(Field::len).sum::<usize>(),
        times,
        rayon::current_num_threads()
    );
    // The same work split into its two passes, so each can be timed on its own.
    let (mut screen_s, mut full_s) = (0.0, 0.0);
    for _ in 0..reps {
        for part in &parts {
            let t = Instant::now();
            let screened =
                sight_core::screen::screen(part, &dem, &prep.templates, &cfg.score, &cfg.screen);
            screen_s += t.elapsed().as_secs_f64();
            let t = Instant::now();
            let keep = cfg
                .screen
                .keep_min
                .max((cfg.screen.keep_share * part.len() as f64) as usize);
            let chosen = &screened[..keep.min(screened.len())];
            let sub = part.subset(&chosen.iter().map(|r| r.index).collect::<Vec<_>>());
            let yaws: Vec<Vec<u16>> = chosen
                .iter()
                .map(|r| sight_core::score::yaws_around(r.yaw, cfg.screen.reach))
                .collect();
            let _ = sight_core::score::surface(
                &sub,
                &dem,
                &prep.lenses,
                sight_core::score::Yaws::PerPlace(&yaws),
                &cfg.score,
            );
            full_s += t.elapsed().as_secs_f64();
        }
    }
    let n = reps.max(1) as f64;
    println!(
        "per rep: screen {:.3} s, full search on survivors {:.3} s",
        screen_s / n,
        full_s / n
    );
    Ok(())
}

fn compare_fields(a: &Path, b: &Path) -> Result<()> {
    let (fa, fb) = (sight_io::read_field(a)?, sight_io::read_field(b)?);
    if fa.len() != fb.len() {
        bail!("place counts differ: {} vs {}", fa.len(), fb.len());
    }
    let pose = fa.lats.iter().zip(&fb.lats).filter(|(x, y)| x != y).count()
        + fa.lons.iter().zip(&fb.lons).filter(|(x, y)| x != y).count()
        + fa.grounds
            .iter()
            .zip(&fb.grounds)
            .filter(|(x, y)| x != y)
            .count();
    let (mut angles, mut max_angle, mut dists) = (0usize, 0i32, 0usize);
    for (x, y) in fa.angles.iter().zip(&fb.angles) {
        if x != y {
            angles += 1;
            max_angle = max_angle.max((i32::from(*x) - i32::from(*y)).abs());
        }
    }
    for (x, y) in fa.distances.iter().zip(&fb.distances) {
        if (x - y).abs() > 0.01 {
            dists += 1;
        }
    }
    println!(
        "places {} values {} pose diffs {pose} angle diffs {angles} (max {max_angle} centideg) distance diffs {dists}",
        fa.len(),
        fa.len() * AZIMUTHS
    );
    Ok(())
}

#[derive(serde::Deserialize)]
struct GoldenRow {
    lat: f64,
    lon: f64,
    yaw: f64,
    fov: f64,
    roll: f64,
    pitch: f64,
    rmse: f64,
    loss: f64,
}

fn golden(
    grid: &Path,
    field_path: &Path,
    image: &Path,
    golden: &Path,
    skyline: Option<&Path>,
) -> Result<()> {
    let cfg = Config::default();
    let loaded = LoadedDem::open(grid)?;
    let dem = loaded.dem();
    let whole = sight_io::read_field(field_path)?;
    let picks = JavaRandom::new(7).distinct_sorted(whole.len() as i32, 60);
    let part = whole.subset(&picks);
    let (photo, _) = photo(image, skyline, &cfg)?;
    let prep = search::Prepared::new(&photo, &cfg);
    let mut rows = sight_core::score::surface(
        &part,
        &dem,
        &prep.lenses,
        sight_core::score::Yaws::All,
        &cfg.score,
    );
    rows.sort_by(|a, b| a.lat.total_cmp(&b.lat).then(a.lon.total_cmp(&b.lon)));
    let want: Vec<GoldenRow> = serde_json::from_str(&std::fs::read_to_string(golden)?)?;
    if want.len() != rows.len() {
        bail!("{} rows, golden has {}", rows.len(), want.len());
    }
    let (mut exact, mut worst) = (0usize, 0.0f64);
    for (g, r) in want.iter().zip(&rows) {
        let same = g.lat == r.lat
            && g.lon == r.lon
            && g.yaw == r.yaw
            && g.fov == r.fov
            && g.roll == r.roll;
        let diff = (g.pitch - r.pitch)
            .abs()
            .max((g.rmse - r.rmse).abs())
            .max((g.loss - r.loss).abs());
        worst = worst.max(diff);
        if same && diff < 1e-9 {
            exact += 1;
        } else {
            println!(
                "differs at {},{}: want yaw {} fov {} roll {} loss {} / got yaw {} fov {} roll {} loss {}",
                g.lat, g.lon, g.yaw, g.fov, g.roll, g.loss, r.yaw, r.fov, r.roll, r.loss
            );
        }
    }
    println!(
        "golden: {exact}/{} match, worst numeric difference {worst:e}",
        rows.len()
    );
    Ok(())
}

fn compare_trace(image: &Path, skyline: &Path) -> Result<()> {
    let img = sight_io::read_image(image)?;
    let ours = sky::trace(&img, &Config::default().sky);
    let theirs = sight_io::read_skyline(skyline)?;
    let at = |line: &[sight_core::camera::Pixel]| -> std::collections::BTreeMap<i64, f64> {
        line.iter().map(|p| (p.x.floor() as i64, p.y)).collect()
    };
    let (a, b) = (at(&ours), at(&theirs));
    let both = a.keys().filter(|k| b.contains_key(k)).count();
    let differ: Vec<f64> = a
        .iter()
        .filter_map(|(k, y)| b.get(k).map(|z| (y - z).abs()))
        .filter(|d| *d > 0.0)
        .collect();
    let worst = differ.iter().copied().fold(0.0, f64::max);
    println!(
        "ours {} theirs {} shared columns {both} differing {} (worst {worst} px)",
        ours.len(),
        theirs.len(),
        differ.len()
    );
    Ok(())
}
