//! The horizon field: one record per place, built once and read on every search. In memory it
//! is a structure of arrays, so the scoring loops walk contiguous memory. On disk it is the
//! `HZN2` format: a 12-byte header (magic, count, azimuths), then per place a little-endian f64
//! lat, f64 lon, f32 ground, 360 i16 centidegree angles and 360 f32 meters.

use rayon::prelude::*;

use crate::config::HorizonConfig;
use crate::dem::Dem;
use crate::horizon::{self, AZIMUTHS, Horizon};

pub const MAGIC: &[u8; 4] = b"HZN2";
const HEADER_BYTES: usize = 12;
const POSE_BYTES: usize = 20;
const RECORD_BYTES: usize = POSE_BYTES + AZIMUTHS * 2 + AZIMUTHS * 4;

/// Places and their horizons, structure-of-arrays. `angles` and `distances` are place-major,
/// `AZIMUTHS` per place.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Field {
    pub lats: Vec<f64>,
    pub lons: Vec<f64>,
    pub grounds: Vec<f32>,
    pub angles: Vec<i16>,
    pub distances: Vec<f32>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FieldError {
    #[error("not a horizon field: magic {0:?}")]
    BadMagic([u8; 4]),
    #[error("field stores {0} azimuths, this build reads {AZIMUTHS}")]
    Azimuths(i32),
    #[error("field is truncated: {have} bytes, {want} needed")]
    Truncated { have: usize, want: usize },
}

impl Field {
    #[must_use]
    pub fn len(&self) -> usize {
        self.lats.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lats.is_empty()
    }

    /// The 360 centidegree angles of place `i`.
    #[inline]
    #[must_use]
    pub fn angles_of(&self, i: usize) -> &[i16] {
        &self.angles[i * AZIMUTHS..(i + 1) * AZIMUTHS]
    }

    /// The 360 distances (m) of place `i`.
    #[inline]
    #[must_use]
    pub fn distances_of(&self, i: usize) -> &[f32] {
        &self.distances[i * AZIMUTHS..(i + 1) * AZIMUTHS]
    }

    fn push(&mut self, lat: f64, lon: f64, ground: f32, h: &Horizon) {
        self.lats.push(lat);
        self.lons.push(lon);
        self.grounds.push(ground);
        self.angles.extend_from_slice(&h.angles);
        self.distances.extend_from_slice(&h.distances);
    }

    /// The places at `indices`, in that order, as a field of their own.
    #[must_use]
    pub fn subset(&self, indices: &[usize]) -> Self {
        let mut out = Self::default();
        out.lats.reserve(indices.len());
        for &i in indices {
            out.lats.push(self.lats[i]);
            out.lons.push(self.lons[i]);
            out.grounds.push(self.grounds[i]);
            out.angles.extend_from_slice(self.angles_of(i));
            out.distances.extend_from_slice(self.distances_of(i));
        }
        out
    }

    /// Reads an `HZN2` field.
    ///
    /// # Errors
    /// When the magic, azimuth count or length is wrong.
    pub fn parse(bytes: &[u8]) -> Result<Self, FieldError> {
        let need = |want: usize| {
            if bytes.len() < want {
                Err(FieldError::Truncated {
                    have: bytes.len(),
                    want,
                })
            } else {
                Ok(())
            }
        };
        need(HEADER_BYTES)?;
        let magic = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if &magic != MAGIC {
            return Err(FieldError::BadMagic(magic));
        }
        let count = i32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let azimuths = i32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        if azimuths as usize != AZIMUTHS {
            return Err(FieldError::Azimuths(azimuths));
        }
        let n = count.max(0) as usize;
        need(HEADER_BYTES + n * RECORD_BYTES)?;
        let mut out = Self {
            lats: Vec::with_capacity(n),
            lons: Vec::with_capacity(n),
            grounds: Vec::with_capacity(n),
            angles: Vec::with_capacity(n * AZIMUTHS),
            distances: Vec::with_capacity(n * AZIMUTHS),
        };
        for rec in bytes[HEADER_BYTES..HEADER_BYTES + n * RECORD_BYTES].chunks_exact(RECORD_BYTES) {
            out.lats.push(f64::from_le_bytes(array8(&rec[0..8])));
            out.lons.push(f64::from_le_bytes(array8(&rec[8..16])));
            out.grounds.push(f32::from_le_bytes(array4(&rec[16..20])));
            let angles = &rec[POSE_BYTES..POSE_BYTES + AZIMUTHS * 2];
            out.angles.extend(
                angles
                    .chunks_exact(2)
                    .map(|c| i16::from_le_bytes([c[0], c[1]])),
            );
            let dists = &rec[POSE_BYTES + AZIMUTHS * 2..];
            out.distances.extend(
                dists
                    .chunks_exact(4)
                    .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])),
            );
        }
        Ok(out)
    }

    /// The field as `HZN2` bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let n = self.len();
        let mut out = Vec::with_capacity(HEADER_BYTES + n * RECORD_BYTES);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&(n as i32).to_le_bytes());
        out.extend_from_slice(&(AZIMUTHS as i32).to_le_bytes());
        for i in 0..n {
            out.extend_from_slice(&self.lats[i].to_le_bytes());
            out.extend_from_slice(&self.lons[i].to_le_bytes());
            out.extend_from_slice(&self.grounds[i].to_le_bytes());
            for a in self.angles_of(i) {
                out.extend_from_slice(&a.to_le_bytes());
            }
            for d in self.distances_of(i) {
                out.extend_from_slice(&d.to_le_bytes());
            }
        }
        out
    }
}

fn array8(b: &[u8]) -> [u8; 8] {
    let mut a = [0; 8];
    a.copy_from_slice(b);
    a
}

fn array4(b: &[u8]) -> [u8; 4] {
    let mut a = [0; 4];
    a.copy_from_slice(b);
    a
}

/// `start, start + step, …` while below `end`, by repeated addition: the reference's double
/// range accumulates the same way, so the lattice lands on the same bits.
fn steps(start: f64, end: f64, step: f64) -> Vec<f64> {
    let mut out = Vec::new();
    let mut v = start;
    while v < end {
        out.push(v);
        v += step;
    }
    out
}

/// Places on a `step_m` grid inside the box, as (lat, lon), rows south to north. The step is in
/// meters of latitude; longitude is stretched by cos of the box's middle so cells stay nearly
/// square.
#[must_use]
pub fn lattice(south: f64, north: f64, west: f64, east: f64, step_m: f64) -> Vec<(f64, f64)> {
    #[allow(
        clippy::manual_midpoint,
        reason = "the reference computes (a + b) / 2; the lattice must land on the same bits"
    )]
    let mid = (south + north) / 2.0;
    let lon_m = horizon::LAT_M * mid.to_radians().cos();
    let dlat = step_m / horizon::LAT_M;
    let dlon = step_m / lon_m;
    let lons = steps(west + dlon / 2.0, east, dlon);
    steps(south + dlat / 2.0, north, dlat)
        .into_iter()
        .flat_map(|lat| lons.iter().map(move |&lon| (lat, lon)))
        .collect()
}

/// Places within `reach_m` of `lat`, `lon` on a `step_m` grid, as (lat, lon).
#[must_use]
pub fn around(lat: f64, lon: f64, step_m: f64, reach_m: f64) -> Vec<(f64, f64)> {
    let lon_m = horizon::LAT_M * lat.to_radians().cos();
    let n = (reach_m / step_m) as i64;
    (-n..=n)
        .flat_map(|i| {
            (-n..=n).map(move |j| {
                (
                    lat + (i as f64 * step_m) / horizon::LAT_M,
                    lon + (j as f64 * step_m) / lon_m,
                )
            })
        })
        .collect()
}

/// The field for `places`, walked now and in parallel. A place with no ground is left out;
/// the rest keep their order.
#[must_use]
pub fn gather(dem: &Dem<'_>, cfg: &HorizonConfig, places: &[(f64, f64)]) -> Field {
    type Walked = Option<(f64, f64, f32, Box<Horizon>)>;
    let walked: Vec<Walked> = places
        .par_iter()
        .map(|&(lat, lon)| {
            let ground = dem.elev_at(lat, lon)?;
            let mut h = Box::<Horizon>::default();
            horizon::view_into(dem, cfg, lat, lon, &mut h).then_some((lat, lon, ground as f32, h))
        })
        .collect();
    let mut out = Field::default();
    for (lat, lon, ground, h) in walked.into_iter().flatten() {
        out.push(lat, lon, ground, &h);
    }
    out
}

/// Whether the ground within `radius_m` of the place spans at least `relief_m`: among such
/// ground the skyline moves by degrees within a few hundred meters, so the field there needs a
/// finer lattice than the open valley.
#[must_use]
pub fn rugged(dem: &Dem<'_>, lat: f64, lon: f64, radius_m: f64, relief_m: f64) -> bool {
    let lon_m = horizon::LAT_M * lat.to_radians().cos();
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for r in [0.0, radius_m / 3.0, 2.0 * (radius_m / 3.0), radius_m] {
        for k in 0..16 {
            let t = (f64::from(k) * 22.5).to_radians();
            if let Some(z) = dem.elev_at(
                lat + (r * t.cos()) / horizon::LAT_M,
                lon + (r * t.sin()) / lon_m,
            ) {
                lo = lo.min(z);
                hi = hi.max(z);
            }
        }
    }
    hi.is_finite() && hi - lo >= relief_m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_round_trip() {
        let mut f = Field::default();
        let mut h = Horizon::default();
        h.angles[3] = 123;
        h.distances[3] = 4567.5;
        f.push(40.5, -111.8, 1500.25, &h);
        f.push(40.6, -111.7, 1600.0, &Horizon::default());
        let back = Field::parse(&f.to_bytes()).unwrap();
        assert_eq!(back, f);
    }

    #[test]
    fn subset_keeps_order() {
        let mut f = Field::default();
        for i in 0..3 {
            let mut h = Horizon::default();
            h.angles[0] = i;
            f.push(f64::from(i), 0.0, 0.0, &h);
        }
        let s = f.subset(&[2, 0]);
        assert_eq!(s.lats, vec![2.0, 0.0]);
        assert_eq!(s.angles_of(0)[0], 2);
    }

    #[test]
    fn around_is_square_and_centered() {
        let pts = around(40.0, -111.0, 100.0, 300.0);
        assert_eq!(pts.len(), 49);
        assert_eq!(pts[24], (40.0, -111.0));
    }
}
