//! A bare-earth grid: a header and row-major i16 elevations in meters, row 0 at the north edge.
//! Off the grid and nodata are `None`, never zero, so a horizon ray can tell it has left the
//! loaded land.

use std::collections::BTreeMap;

/// Stored where the source has no ground; never a real Utah elevation.
pub const NODATA: i16 = i16::MIN;

/// Where the grid sits and how its cells are spaced.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Georef {
    /// Latitude of the north edge.
    pub north: f64,
    /// Longitude of the west edge.
    pub west: f64,
    /// Degrees of longitude per column.
    pub dx: f64,
    /// Degrees of latitude per row, negative: rows run south.
    pub dy: f64,
    pub width: usize,
    pub height: usize,
}

/// A grid borrowing its samples, so a memory-mapped file is used without copying.
#[derive(Clone, Copy, Debug)]
pub struct Dem<'a> {
    pub geo: Georef,
    pub samples: &'a [i16],
}

/// The byte order of the samples on disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteOrder {
    Little,
    Big,
}

/// An ENVI header, parsed: the grid's placement and how to read its samples.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnviHeader {
    pub geo: Georef,
    pub byte_order: ByteOrder,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum DemError {
    #[error("ENVI grid is not int16 (data type {0:?})")]
    NotInt16(Option<String>),
    #[error("ENVI grid is not band sequential (interleave {0:?})")]
    NotBsq(Option<String>),
    #[error("ENVI header has no usable {0}")]
    Missing(&'static str),
    #[error("ENVI map info has no tie point: {0}")]
    NoTiePoint(String),
    #[error("grid has {have} samples, header says {want}")]
    SizeMismatch { have: usize, want: usize },
}

impl EnviHeader {
    /// Parses ENVI header text: `key = value` lines, keys lower-cased. Only int16, band
    /// sequential grids are accepted.
    ///
    /// # Errors
    /// When the data type, interleave, size or map info is missing or unsupported.
    pub fn parse(text: &str) -> Result<Self, DemError> {
        let entries: BTreeMap<String, String> = text
            .lines()
            .filter_map(|line| {
                let (k, v) = line.split_once('=')?;
                Some((k.trim().to_lowercase(), v.trim().to_string()))
            })
            .collect();
        let get = |k: &str| entries.get(k).cloned();
        if get("data type").as_deref() != Some("2") {
            return Err(DemError::NotInt16(get("data type")));
        }
        if get("interleave").map(|s| s.to_lowercase()).as_deref() != Some("bsq") {
            return Err(DemError::NotBsq(get("interleave")));
        }
        let width = get("samples")
            .and_then(|s| s.parse().ok())
            .ok_or(DemError::Missing("samples"))?;
        let height = get("lines")
            .and_then(|s| s.parse().ok())
            .ok_or(DemError::Missing("lines"))?;
        let info = get("map info").ok_or(DemError::Missing("map info"))?;
        let nums = numbers(&info);
        if nums.len() < 6 {
            return Err(DemError::NoTiePoint(info));
        }
        let byte_order = if get("byte order").as_deref() == Some("1") {
            ByteOrder::Big
        } else {
            ByteOrder::Little
        };
        Ok(Self {
            geo: Georef {
                west: nums[2],
                north: nums[3],
                dx: nums[4],
                dy: -nums[5].abs(),
                width,
                height,
            },
            byte_order,
        })
    }
}

/// Every number in `text`, in order: the reference reads ENVI's `map info` with the regex
/// `-?[0-9]+(?:\.[0-9]+)?(?:[eE][-+]?[0-9]+)?`, and this scanner accepts the same strings.
fn numbers(text: &str) -> Vec<f64> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let start = i;
        let mut j = i;
        if b[j] == b'-' {
            j += 1;
        }
        let digits = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j == digits {
            i += 1;
            continue;
        }
        if j + 1 < b.len() && b[j] == b'.' && b[j + 1].is_ascii_digit() {
            j += 1;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
        }
        if j < b.len() && (b[j] == b'e' || b[j] == b'E') {
            let mut k = j + 1;
            if k < b.len() && (b[k] == b'+' || b[k] == b'-') {
                k += 1;
            }
            let exp = k;
            while k < b.len() && b[k].is_ascii_digit() {
                k += 1;
            }
            if k > exp {
                j = k;
            }
        }
        if let Ok(v) = text[start..j].parse() {
            out.push(v);
        }
        i = j;
    }
    out
}

impl<'a> Dem<'a> {
    /// A grid over `samples`, checked against the header's size.
    ///
    /// # Errors
    /// When the sample count is not width × height.
    pub fn new(geo: Georef, samples: &'a [i16]) -> Result<Self, DemError> {
        let want = geo.width * geo.height;
        if samples.len() != want {
            return Err(DemError::SizeMismatch {
                have: samples.len(),
                want,
            });
        }
        Ok(Self { geo, samples })
    }

    /// Latitude of the south edge.
    #[must_use]
    pub fn south(&self) -> f64 {
        self.geo.north + self.geo.dy * self.geo.height as f64
    }

    /// Longitude of the east edge.
    #[must_use]
    pub fn east(&self) -> f64 {
        self.geo.west + self.geo.dx * self.geo.width as f64
    }

    fn sample(&self, row: i64, col: i64) -> i16 {
        if row < 0 || col < 0 || row >= self.geo.height as i64 || col >= self.geo.width as i64 {
            return NODATA;
        }
        self.samples[row as usize * self.geo.width + col as usize]
    }

    /// The sample at the cell containing `row`, `col` rounded the reference's way, or `NODATA`.
    #[inline]
    #[must_use]
    pub fn nearest(&self, row: i64, col: i64) -> i16 {
        self.sample(row, col)
    }

    /// Meters at `lat`, `lon`, bilinear between the four surrounding cells, or `None` if any
    /// of them is missing or the point is off the grid.
    #[must_use]
    pub fn elev_at(&self, lat: f64, lon: f64) -> Option<f64> {
        let g = &self.geo;
        let col = (lon - g.west) / g.dx;
        let row = (lat - g.north) / g.dy;
        if !(col >= 0.0
            && row >= 0.0
            && col < (g.width as f64 - 1.0)
            && row < (g.height as f64 - 1.0))
        {
            return None;
        }
        let c0 = col.floor() as i64;
        let r0 = row.floor() as i64;
        let tc = col - c0 as f64;
        let tr = row - r0 as f64;
        let v00 = self.sample(r0, c0);
        let v10 = self.sample(r0, c0 + 1);
        let v01 = self.sample(r0 + 1, c0);
        let v11 = self.sample(r0 + 1, c0 + 1);
        if v00 == NODATA || v10 == NODATA || v01 == NODATA || v11 == NODATA {
            return None;
        }
        let (v00, v10, v01, v11) = (
            f64::from(v00),
            f64::from(v10),
            f64::from(v01),
            f64::from(v11),
        );
        Some((1.0 - tr) * ((1.0 - tc) * v00 + tc * v10) + tr * ((1.0 - tc) * v01 + tc * v11))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HDR: &str = "ENVI\nsamples = 3\nlines   = 3\nbands = 1\ndata type = 2\ninterleave = bsq\nbyte order = 0\nmap info = {Geographic Lat/Lon, 1, 1, -112.0005, 41.0005, 0.5, 0.5,North America 1983}\n";

    #[test]
    fn parses_header() {
        let h = EnviHeader::parse(HDR).unwrap();
        assert_eq!(h.geo.width, 3);
        assert_eq!(h.geo.west, -112.0005);
        assert_eq!(h.geo.north, 41.0005);
        assert_eq!(h.geo.dy, -0.5);
        assert_eq!(h.byte_order, ByteOrder::Little);
    }

    #[test]
    fn bilinear_and_missing() {
        let geo = Georef {
            north: 1.0,
            west: 0.0,
            dx: 1.0,
            dy: -1.0,
            width: 3,
            height: 3,
        };
        let samples = [0, 10, 20, 0, 10, 20, NODATA, 10, 20];
        let dem = Dem::new(geo, &samples).unwrap();
        assert_eq!(dem.elev_at(1.0, 0.5), Some(5.0));
        assert_eq!(dem.elev_at(1.0, 1.5), Some(15.0));
        assert_eq!(dem.elev_at(-0.5, 0.5), None, "touches the nodata cell");
        assert_eq!(dem.elev_at(1.0, -0.1), None, "off the grid");
    }
}
