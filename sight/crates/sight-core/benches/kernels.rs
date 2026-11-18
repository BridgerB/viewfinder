//! Microbenchmarks of the inner loops, on synthetic data shaped like real horizons and
//! skylines. `cargo bench -p sight-core`.

use sight_core::screen::{dot, dot32};

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

/// The same cross terms in f32 lanes.
#[divan::bench(args = [40, 70, 100, 115])]
fn cross_term_f32(bencher: divan::Bencher, m: usize) {
    let (g, e) = data(m);
    let g: Vec<f32> = g.iter().map(|&v| v as f32).collect();
    let e: Vec<f32> = e.iter().map(|&v| v as f32).collect();
    bencher.bench(|| {
        let mut acc = 0.0;
        for b in 0..360 {
            acc += dot32(divan::black_box(&g), divan::black_box(&e), b);
        }
        acc
    });
}
