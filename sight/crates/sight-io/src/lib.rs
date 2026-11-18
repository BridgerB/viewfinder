//! Files in and out: the edge where bytes become the core's plain data and results become files
//! the site reads. Formats match the earlier JVM tools, so existing files and pages keep working.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::missing_errors_doc,
    reason = "pixel and grid arithmetic; every public fn returns IoError and says when in its doc"
)]

use std::fmt::Write as _;
use std::fs::File;
use std::path::{Path, PathBuf};

use memmap2::Mmap;
use serde::Deserialize;
use sight_core::camera::Pixel;
use sight_core::dem::{ByteOrder, Dem, DemError, EnviHeader, Georef};
use sight_core::field::{Field, FieldError};
use sight_core::score::Place;
use sight_core::sky::Image;

#[derive(Debug, thiserror::Error)]
pub enum IoError {
    #[error("{path}: {source}")]
    File {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path}: {source}")]
    Dem { path: PathBuf, source: DemError },
    #[error("{path}: {source}")]
    Field { path: PathBuf, source: FieldError },
    #[error("{path}: {source}")]
    Image {
        path: PathBuf,
        source: image::ImageError,
    },
    #[error("photo: {0}")]
    Photo(image::ImageError),
    #[error("{path}: {source}")]
    Toml {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("{path}: expected {want} bytes of samples, found {have}")]
    Size {
        path: PathBuf,
        want: usize,
        have: usize,
    },
}

fn file_err(path: &Path) -> impl FnOnce(std::io::Error) -> IoError + '_ {
    move |source| IoError::File {
        path: path.to_path_buf(),
        source,
    }
}

enum Samples {
    /// Mapped straight from disk: the file is little-endian, like this machine.
    Mapped(Mmap),
    /// Converted from a big-endian file.
    Owned(Vec<i16>),
}

/// A DEM loaded from an ENVI pair (`path` and `path.hdr`), memory-mapped when the byte order
/// allows, so loading costs nothing until cells are touched.
pub struct LoadedDem {
    geo: Georef,
    samples: Samples,
}

impl LoadedDem {
    pub fn open(path: &Path) -> Result<Self, IoError> {
        let hdr_path = PathBuf::from(format!("{}.hdr", path.display()));
        let text = std::fs::read_to_string(&hdr_path).map_err(file_err(&hdr_path))?;
        let header = EnviHeader::parse(&text).map_err(|source| IoError::Dem {
            path: hdr_path.clone(),
            source,
        })?;
        let want = header.geo.width * header.geo.height * 2;
        let file = File::open(path).map_err(file_err(path))?;
        // SAFETY: the map is read-only and the grid files are written once by the conversion
        // step and never modified while a search runs. If another process truncated the file
        // the map would fault rather than return wrong data.
        #[allow(
            unsafe_code,
            reason = "memory-mapping the grid is the point: 234 MB without a copy"
        )]
        let map = unsafe { Mmap::map(&file) }.map_err(file_err(path))?;
        if map.len() != want {
            return Err(IoError::Size {
                path: path.to_path_buf(),
                want,
                have: map.len(),
            });
        }
        let little_host = cfg!(target_endian = "little");
        let samples = match header.byte_order {
            ByteOrder::Little
                if little_host && bytemuck::try_cast_slice::<u8, i16>(&map).is_ok() =>
            {
                Samples::Mapped(map)
            }
            ByteOrder::Little => Samples::Owned(
                map.chunks_exact(2)
                    .map(|c| i16::from_le_bytes([c[0], c[1]]))
                    .collect(),
            ),
            ByteOrder::Big => Samples::Owned(
                map.chunks_exact(2)
                    .map(|c| i16::from_be_bytes([c[0], c[1]]))
                    .collect(),
            ),
        };
        Ok(Self {
            geo: header.geo,
            samples,
        })
    }

    /// The grid, borrowing the mapped or converted samples.
    #[must_use]
    pub fn dem(&self) -> Dem<'_> {
        let samples: &[i16] = match &self.samples {
            Samples::Mapped(map) => bytemuck::cast_slice(map),
            Samples::Owned(v) => v,
        };
        Dem {
            geo: self.geo,
            samples,
        }
    }
}

pub fn read_field(path: &Path) -> Result<Field, IoError> {
    let bytes = std::fs::read(path).map_err(file_err(path))?;
    Field::parse(&bytes).map_err(|source| IoError::Field {
        path: path.to_path_buf(),
        source,
    })
}

pub fn write_field(path: &Path, field: &Field) -> Result<(), IoError> {
    std::fs::write(path, field.to_bytes()).map_err(file_err(path))
}

/// Several fields given as a comma-separated list of paths.
pub fn read_fields(list: &str) -> Result<Vec<Field>, IoError> {
    list.split(',').map(|p| read_field(Path::new(p))).collect()
}

fn argb_of(img: &image::RgbImage) -> Image {
    let (width, height) = (img.width() as usize, img.height() as usize);
    let argb = img
        .pixels()
        .map(|p| 0xff00_0000 | (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2]))
        .collect();
    Image {
        width,
        height,
        argb,
    }
}

/// A JPEG or PNG decoded to ARGB.
pub fn read_image(path: &Path) -> Result<Image, IoError> {
    let img = image::open(path)
        .map_err(|source| IoError::Image {
            path: path.to_path_buf(),
            source,
        })?
        .to_rgb8();
    Ok(argb_of(&img))
}

/// A photograph decoded from uploaded bytes.
pub struct Decoded {
    /// Pixels turned upright by the EXIF orientation, for tracing.
    pub image: Image,
    /// The same upright picture as a JPEG, so a page draws skylines in the same coordinates.
    pub jpeg: Vec<u8>,
    /// Horizontal field of view in degrees, when EXIF gives the 35 mm-equivalent focal length.
    pub fov: Option<f64>,
}

/// Decodes a JPEG or PNG, applies its EXIF orientation (phones store many photos sideways and
/// let the viewer rotate them), and reads the lens from EXIF.
pub fn decode_photo(bytes: &[u8]) -> Result<Decoded, IoError> {
    use image::ImageDecoder as _;
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| IoError::Photo(image::ImageError::IoError(e)))?;
    let mut decoder = reader.into_decoder().map_err(IoError::Photo)?;
    let orientation = decoder.orientation().map_err(IoError::Photo)?;
    let mut img = image::DynamicImage::from_decoder(decoder).map_err(IoError::Photo)?;
    img.apply_orientation(orientation);
    let rgb = img.to_rgb8();
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 88)
        .encode_image(&rgb)
        .map_err(IoError::Photo)?;
    Ok(Decoded {
        image: argb_of(&rgb),
        jpeg,
        fov: exif_fov(bytes),
    })
}

/// Horizontal field of view from EXIF `FocalLengthIn35mmFilm` (JPEG or HEIF): 2·atan(18 / f),
/// half of a 36 mm frame over the focal length.
#[must_use]
pub fn exif_fov(bytes: &[u8]) -> Option<f64> {
    let exif = exif::Reader::new()
        .read_from_container(&mut std::io::Cursor::new(bytes))
        .ok()?;
    let focal = exif
        .get_field(exif::Tag::FocalLengthIn35mmFilm, exif::In::PRIMARY)?
        .value
        .get_uint(0)?;
    (focal > 0).then(|| 2.0 * (18.0 / f64::from(focal)).atan().to_degrees())
}

/// Skyline pixels from any file of `[x, y]` pairs (a hand-guided skyline, or a page's JSON).
pub fn read_skyline(path: &Path) -> Result<Vec<Pixel>, IoError> {
    let text = std::fs::read_to_string(path).map_err(file_err(path))?;
    let pair = regex::Regex::new(r"\[\s*(-?\d+\.?\d*)\s*,\s*(-?\d+\.?\d*)\s*\]").map_err(|e| {
        IoError::File {
            path: path.to_path_buf(),
            source: std::io::Error::other(e),
        }
    })?;
    Ok(pair
        .captures_iter(&text)
        .filter_map(|c| {
            Some(Pixel {
                x: c[1].parse().ok()?,
                y: c[2].parse().ok()?,
            })
        })
        .collect())
}

/// Every scored place as `[{"lat","lon","loss"}]`, the heatmap the match page draws.
pub fn write_surface(path: &Path, rows: &[Place]) -> Result<(), IoError> {
    let mut s = String::from("[");
    for (i, r) in rows.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let _ = write!(
            s,
            "{{\"lat\":{:.5},\"lon\":{:.5},\"loss\":{:.4}}}",
            r.lat, r.lon, r.loss
        );
    }
    s.push(']');
    std::fs::write(path, s).map_err(file_err(path))
}

/// The winning place, its camera, and its horizon as pixels of the photograph.
pub fn write_match(
    path: &Path,
    p: &Place,
    outline: &[(f64, f64)],
    width: u32,
    height: u32,
) -> Result<(), IoError> {
    let mut s = format!(
        "{{\"width\":{width},\"height\":{height},\"lat\":{:.5},\"lon\":{:.5},\"elev\":{:.0},\"yaw\":{:.0},\"fov\":{:.0},\"roll\":{:.0},\"pitch\":{:.2},\"rmse\":{:.3},\"corr\":{:.3},\"loss\":{:.4},\"horizon\":[",
        p.lat, p.lon, p.elev, p.yaw, p.fov, p.roll, p.pitch, p.rmse, p.corr, p.loss
    );
    for (i, (x, y)) in outline.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let _ = write!(s, "[{x:.1},{y:.1}]");
    }
    s.push_str("]}");
    std::fs::write(path, s).map_err(file_err(path))
}

/// Ridges per camera id as `{"id":[[lat,lon],…]}`.
pub fn write_ridges(path: &Path, ridges: &[(String, Vec<(f64, f64)>)]) -> Result<(), IoError> {
    let mut s = String::from("{");
    for (i, (id, pts)) in ridges.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let _ = write!(s, "\"{id}\":[");
        for (j, (a, b)) in pts.iter().enumerate() {
            if j > 0 {
                s.push(',');
            }
            let _ = write!(s, "[{a:.5},{b:.5}]");
        }
        s.push(']');
    }
    s.push('}');
    std::fs::write(path, s).map_err(file_err(path))
}

fn stamp(img: &mut image::RgbImage, x: f64, y: f64, radius: f64, color: image::Rgb<u8>) {
    let r = radius.ceil() as i64;
    let (cx, cy) = (x.round() as i64, y.round() as i64);
    for dy in -r..=r {
        for dx in -r..=r {
            if ((dx * dx + dy * dy) as f64) <= radius * radius {
                let (px, py) = (cx + dx, cy + dy);
                if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
                    img.put_pixel(px as u32, py as u32, color);
                }
            }
        }
    }
}

fn polyline(img: &mut image::RgbImage, points: &[(f64, f64)], thick: f64, color: image::Rgb<u8>) {
    for w in points.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        let len = (x1 - x0).hypot(y1 - y0);
        let steps = (len * 2.0).ceil().max(1.0) as usize;
        for k in 0..=steps {
            let t = k as f64 / steps as f64;
            stamp(
                img,
                x0 + t * (x1 - x0),
                y0 + t * (y1 - y0),
                thick / 2.0,
                color,
            );
        }
    }
}

/// The photograph with the skyline in orange and the winner's horizon in cyan, as a JPEG.
pub fn write_overlay(
    image: &Path,
    out: &Path,
    skyline: &[Pixel],
    outline: &[(f64, f64)],
) -> Result<(), IoError> {
    let mut img = image::open(image)
        .map_err(|source| IoError::Image {
            path: image.to_path_buf(),
            source,
        })?
        .to_rgb8();
    let thick = 2.0_f64.max(f64::from(img.width()) / 500.0);
    let sky: Vec<(f64, f64)> = skyline.iter().map(|p| (p.x, p.y)).collect();
    polyline(&mut img, &sky, thick, image::Rgb([255, 176, 32]));
    polyline(&mut img, outline, thick, image::Rgb([62, 224, 224]));
    img.save(out).map_err(|source| IoError::Image {
        path: out.to_path_buf(),
        source,
    })
}

/// One known-location photograph in the bench.
#[derive(Clone, Debug, Deserialize)]
pub struct BenchCase {
    pub id: String,
    pub image: PathBuf,
    pub skyline: Option<PathBuf>,
    pub fov: Option<f64>,
    pub truth: [f64; 2],
    pub baseline_km: f64,
    #[serde(default)]
    pub quick: bool,
}

#[derive(Deserialize)]
struct Cases {
    case: Vec<BenchCase>,
}

pub fn read_cases(path: &Path) -> Result<Vec<BenchCase>, IoError> {
    let text = std::fs::read_to_string(path).map_err(file_err(path))?;
    let cases: Cases = toml::from_str(&text).map_err(|source| IoError::Toml {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(cases.case)
}
