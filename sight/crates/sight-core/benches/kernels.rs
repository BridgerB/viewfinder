//! Microbenchmarks of the inner loops, on synthetic data shaped like real horizons and
//! skylines. `cargo bench -p sight-core`.

use sight_core::camera::{Photo, Pixel};
use sight_core::config::{ScoreConfig, ScreenConfig};
use sight_core::dem::{Dem, Georef};
use sight_core::field::Field;
use sight_core::horizon::AZIMUTHS;
use sight_core::screen::{cross_all, dot, screen, templates};

fn main() {
    divan::main();
}

fn data(m: usize) -> (Vec<f64>, Vec<f64>) {
    let g: Vec<f64> = (0..m).map(|k| 5.0 + (k as f64 * 0.37).sin()).collect();
    let e: Vec<f64> = (0..720).map(|j| 4.0 + (j as f64 * 0.11).cos() * 2.0).collect();
    (g, e)
}

/// One window's cross term, at the template widths the screen uses.
#[divan::bench(args = [40, 70, 100, 115])]
fn cross_term(bencher: divan::Bencher, m: usize) {
    let (g, e) = data(m);
    bencher.bench(|| {
        let mut acc = 0.0;
        for b in 0..360 {
            acc += dot(divan::black_box(&g), divan::black_box(&e), b);
        }
        acc
    });
}

/// The whole screen, single-threaded, over 64 places with ridged horizons on flat ground: every
/// template, every yaw, the window algebra and the foreground drop. Time per place is this / 64.
#[divan::bench(sample_count = 30)]
fn screen_64_places(bencher: divan::Bencher) {
    let n = 400;
    let samples = vec![1500_i16; n * n];
    let geo = Georef { north: 40.6, west: -111.9, dx: 1e-4, dy: -1e-4, width: n, height: n };
    let dem = Dem::new(geo, &samples).unwrap_or_else(|_| std::process::abort());
    let mut field = Field::default();
    for i in 0..64 {
        let p = f64::from(i);
        field.lats.push(40.58 - p * 2e-4);
        field.lons.push(-111.88 + p * 2e-4);
        field.grounds.push(1500.0);
        field.angles.extend((0..AZIMUTHS).map(|az| {
            let a = (az as f64).to_radians();
            (200.0 + 300.0 * (3.0 * a + p).sin() + 150.0 * (7.0 * a + 2.0 * p).sin()) as i16
        }));
        field.distances.extend(std::iter::repeat_n(8000.0_f32, AZIMUTHS));
    }
    let skyline: Vec<Pixel> = (0..160)
        .map(|k| {
            let x = 12.5 + 25.0 * f64::from(k);
            Pixel { x, y: 1200.0 + 150.0 * (x / 300.0).sin() + 60.0 * (x / 97.0).cos() }
        })
        .collect();
    let photo = Photo { width: 4000, height: 3000, skyline };
    let (score, scr) = (ScoreConfig::default(), ScreenConfig::default());
    let tpls = templates(&photo, &score, &scr);
    let pool = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap_or_else(|_| std::process::abort());
    bencher.bench(|| pool.install(|| screen(divan::black_box(&field), &dem, &tpls, &score, &scr)));
}

/// The same 360 cross terms as the screen computes them: f32, template index outermost.
#[divan::bench(args = [40, 70, 100, 115])]
fn cross_term_all(bencher: divan::Bencher, m: usize) {
    let (g, e) = data(m);
    let g: Vec<f32> = g.iter().map(|&v| v as f32).collect();
    let e: Vec<f32> = e.iter().map(|&v| v as f32).collect();
    let mut out = [0.0_f32; AZIMUTHS];
    bencher.bench_local(|| {
        cross_all(divan::black_box(&g), divan::black_box(&e), &mut out);
        out[7]
    });
}
