//! Where a mountain photograph was taken.
//!
//! The pipeline: a bare-earth grid ([`dem`]) is walked into a 360° horizon per place
//! ([`horizon`]), packed into a field ([`field`]). A photograph's skyline ([`sky`]) is projected
//! through candidate cameras ([`camera`]). Every place is screened cheaply by sliding the
//! skyline around its horizon ([`screen`]); the best survivors get the full trimmed-error
//! search ([`score`]); [`search`] ties the passes together and refines around the winners.
//!
//! The crate does no I/O and reads no clock: callers hand it bytes and plain data and get
//! plain data back. Data-parallel loops use rayon and collect in index order, so the same
//! inputs always give the same output.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::module_name_repetitions,
    clippy::doc_markdown,
    clippy::float_cmp,
    reason = "numeric kernel code: casts between grid indices and f64 are the domain, and exact float comparisons mirror the reference implementation"
)]

pub mod camera;
pub mod config;
pub mod dem;
pub mod field;
pub mod geo;
pub mod horizon;
pub mod java;
pub mod score;
pub mod screen;
pub mod search;
pub mod sky;
pub mod skymap;

pub use config::Config;
