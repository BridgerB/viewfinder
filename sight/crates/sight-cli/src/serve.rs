//! `sight serve`: the locator as a small local HTTP API, for the site's upload page.
//!
//! The grid and fields are loaded once and stay in memory. A photo is uploaded once
//! (`POST /trace`) and kept, as its upright JPEG and traced skyline, under an id; searches
//! (`POST /locate`) then name that id and may send a lens width. `POST /retrace` with a box
//! retraces the skyline inside it (for photos where trees or poles fool the whole-photo trace),
//! and searches use that skyline from then on.
//! Requests are served one at a time; each search uses every core.

use std::collections::BTreeMap;
use std::io::{Cursor, Read as _};
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use sight_core::Config;
use sight_core::camera::{Photo, Pixel, Pose, outline};
use sight_core::field::Field;
use sight_core::geo::ridge;
use sight_core::horizon;
use sight_core::score::Place;
use sight_core::search::{self, NoTimer};
use sight_core::sky;
use sight_io::LoadedDem;
use tiny_http::{Header, Method, Request, Response, Server};

/// Largest upload accepted, bytes. A phone photo is 2–8 MB.
const MAX_UPLOAD: u64 = 64 << 20;
/// Uploads kept for searching; the oldest is dropped past this.
const KEEP: usize = 8;
/// Points of the traced skyline sent to the page for drawing.
const DRAW_POINTS: usize = 600;
/// Rows of the final ranking returned as candidates.
const TOP: usize = 10;

type Reply = Response<Cursor<Vec<u8>>>;

struct Upload {
    jpeg: Vec<u8>,
    width: u32,
    height: u32,
    /// The whole-photo trace.
    traced: Vec<Pixel>,
    /// The skyline a search uses: the whole-photo trace, or the trace inside the box last given
    /// to /retrace.
    current: Vec<Pixel>,
}

#[derive(Deserialize)]
struct RetraceRequest {
    id: u64,
    /// [x0, y0, x1, y1] in image pixels: trace only inside it. Absent: back to the whole photo.
    crop: Option<[f64; 4]>,
}

#[derive(Serialize)]
struct Retraced {
    skyline: Vec<[f64; 2]>,
    /// Points the trace found, before thinning for drawing.
    points: usize,
}

#[derive(Serialize)]
struct Traced {
    id: u64,
    width: u32,
    height: u32,
    /// Lens width from EXIF, degrees, when the photo records it.
    fov: Option<f64>,
    skyline: Vec<[f64; 2]>,
}

#[derive(Deserialize)]
struct LocateRequest {
    id: u64,
    /// A hand-drawn skyline in image pixels; the traced one when absent.
    skyline: Option<Vec<[f64; 2]>>,
    /// Lens width in degrees; every lens 40–115° is tried when absent.
    fov: Option<f64>,
}

#[derive(Serialize)]
struct Spot {
    lat: f64,
    lon: f64,
    loss: f64,
}

#[derive(Serialize)]
struct Located {
    seconds: f64,
    best: Place,
    top: Vec<Place>,
    /// Every place given the full search, for the heatmap.
    surface: Vec<Spot>,
    /// The terrain horizon from the best place, drawn through its camera: image pixels.
    outline: Vec<[f64; 2]>,
    /// The ground each skyline degree was matched to: [lat, lon].
    ridge: Vec<[f64; 2]>,
    /// The skyline the search used, as sampled: image pixels.
    skyline: Vec<[f64; 2]>,
    counts: Vec<usize>,
}

/// Serves the locator over HTTP on `host:port` until killed.
pub fn serve(grid: &Path, fields: &str, host: &str, port: u16) -> Result<()> {
    let loaded = LoadedDem::open(grid)?;
    let dem = loaded.dem();
    let fields = sight_io::read_fields(fields)?;
    let server = Server::http((host, port)).map_err(|e| anyhow!("listen on {host}:{port}: {e}"))?;
    println!(
        "sight serve: {} places, http://{host}:{port}",
        fields.iter().map(Field::len).sum::<usize>()
    );
    let mut state = State {
        uploads: BTreeMap::new(),
        next: 1,
    };
    for mut req in server.incoming_requests() {
        let reply = route(&mut req, &dem, &fields, &mut state).unwrap_or_else(|e| {
            eprintln!("{} {}: {e:#}", req.method(), req.url());
            json(500, &serde_json::json!({ "error": format!("{e:#}") }))
        });
        // Writing happens on its own thread: a client that stops reading (a browser that dropped
        // an image download, say) blocks only that write, never the next request. A client that
        // went away cannot be answered, so the result of the write is ignored.
        std::thread::spawn(move || {
            let _ = req.respond(reply);
        });
    }
    Ok(())
}

struct State {
    uploads: BTreeMap<u64, Upload>,
    next: u64,
}

fn route(
    req: &mut Request,
    dem: &sight_core::dem::Dem<'_>,
    fields: &[Field],
    state: &mut State,
) -> Result<Reply> {
    let path = req.url().split('?').next().unwrap_or("").to_string();
    match (req.method(), path.as_str()) {
        (Method::Options, _) => Ok(cors(Response::from_data(Vec::new()).with_status_code(204))),
        (Method::Get, "/health") => Ok(json(
            200,
            &serde_json::json!({
                "places": fields.iter().map(Field::len).sum::<usize>(),
                "threads": rayon::current_num_threads(),
            }),
        )),
        (Method::Post, "/trace") => {
            let bytes = body(req)?;
            trace(&bytes, state)
        }
        (Method::Get, p) if p.starts_with("/image/") => {
            let id: u64 = p["/image/".len()..].parse().context("bad image id")?;
            let upload = state.uploads.get(&id).context("no such upload")?;
            Ok(with_type(
                cors(Response::from_data(upload.jpeg.clone())),
                "image/jpeg",
            ))
        }
        (Method::Post, "/retrace") => {
            let ask: RetraceRequest =
                serde_json::from_slice(&body(req)?).context("bad retrace request")?;
            retrace(&ask, state)
        }
        (Method::Post, "/locate") => {
            let ask: LocateRequest =
                serde_json::from_slice(&body(req)?).context("bad locate request")?;
            locate(&ask, dem, fields, state)
        }
        _ => Ok(json(404, &serde_json::json!({ "error": "not found" }))),
    }
}

fn body(req: &mut Request) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    req.as_reader()
        .take(MAX_UPLOAD + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_UPLOAD {
        bail!("upload larger than {} MB", MAX_UPLOAD >> 20);
    }
    Ok(bytes)
}

fn trace(bytes: &[u8], state: &mut State) -> Result<Reply> {
    let id = state.next;
    state.next += 1;
    let decoded = match sight_io::decode_photo(bytes) {
        Ok(d) => d,
        Err(direct) => {
            via_converter(bytes, id).with_context(|| format!("cannot read the photo ({direct})"))?
        }
    };
    let fov = decoded.fov.or_else(|| sight_io::exif_fov(bytes));
    let traced = sky::trace_best(&decoded.image, &Config::default().sky);
    let reply = Traced {
        id,
        width: decoded.image.width as u32,
        height: decoded.image.height as u32,
        fov,
        skyline: points(&sky::decimate(&traced, DRAW_POINTS)),
    };
    state.uploads.insert(
        id,
        Upload {
            jpeg: decoded.jpeg,
            width: reply.width,
            height: reply.height,
            current: traced.clone(),
            traced,
        },
    );
    while state.uploads.len() > KEEP {
        state.uploads.pop_first();
    }
    Ok(json(200, &reply))
}

/// HEIC and anything else the image crate cannot read, converted to JPEG by an outside tool:
/// `sips` on macOS, libheif's `heif-convert` elsewhere (the container installs it). Both write
/// the picture upright with the EXIF orientation reset, so it is not rotated twice.
fn via_converter(bytes: &[u8], tag: u64) -> Result<sight_io::Decoded> {
    let dir = std::env::temp_dir();
    let src = dir.join(format!("sight-upload-{}-{tag}", std::process::id()));
    let out = src.with_extension("jpg");
    std::fs::write(&src, bytes)?;
    let mut command = if cfg!(target_os = "macos") {
        let mut c = std::process::Command::new("/usr/bin/sips");
        c.args(["-s", "format", "jpeg"])
            .arg(&src)
            .arg("--out")
            .arg(&out);
        c
    } else {
        let mut c = std::process::Command::new("heif-convert");
        c.args(["-q", "92"]).arg(&src).arg(&out);
        c
    };
    let run = command.output();
    let _ = std::fs::remove_file(&src);
    let ok = run.is_ok_and(|o| o.status.success());
    let jpeg = if ok { std::fs::read(&out).ok() } else { None };
    let _ = std::fs::remove_file(&out);
    let jpeg = jpeg.context("not an image this server can convert (JPEG, PNG or HEIC)")?;
    Ok(sight_io::decode_photo(&jpeg)?)
}

/// Retraces an upload inside a box (or the whole photo again) and makes that the skyline
/// searches use. The pixels are not kept between requests, so the stored JPEG is decoded again.
fn retrace(ask: &RetraceRequest, state: &mut State) -> Result<Reply> {
    let upload = state
        .uploads
        .get_mut(&ask.id)
        .context("that upload has expired; upload the photo again")?;
    upload.current = match ask.crop {
        None => upload.traced.clone(),
        Some([x0, y0, x1, y1]) => {
            let decoded = sight_io::decode_photo(&upload.jpeg)?;
            let corner = |v: f64| v.max(0.0).round() as usize;
            let region = sky::Region {
                x0: corner(x0.min(x1)),
                y0: corner(y0.min(y1)),
                x1: corner(x0.max(x1)),
                y1: corner(y0.max(y1)),
            };
            let line = sky::trace_region(&decoded.image, region);
            if line.len() < 24 {
                bail!(
                    "no mountain ridge found inside that box; make it wider and include some sky above the ridge"
                );
            }
            line
        }
    };
    Ok(json(
        200,
        &Retraced {
            skyline: points(&sky::decimate(&upload.current, DRAW_POINTS)),
            points: upload.current.len(),
        },
    ))
}

fn locate(
    ask: &LocateRequest,
    dem: &sight_core::dem::Dem<'_>,
    fields: &[Field],
    state: &State,
) -> Result<Reply> {
    let upload = state
        .uploads
        .get(&ask.id)
        .context("that upload has expired; upload the photo again")?;
    let cfg = match ask.fov {
        Some(f) if (10.0..=130.0).contains(&f) => Config::default().with_known_fov(f),
        Some(f) => bail!("lens width {f}° is outside 10–130°"),
        None => Config::default(),
    };
    let line: Vec<Pixel> = match &ask.skyline {
        Some(drawn) => densify(drawn),
        None => upload.current.clone(),
    };
    if line.len() < 24 {
        bail!("the skyline has too few points to place a camera");
    }
    let photo = Photo {
        width: upload.width,
        height: upload.height,
        skyline: sky::decimate(&line, cfg.sky.columns),
    };
    let start = Instant::now();
    let found = search::search(dem, fields, &photo, &cfg, &mut NoTimer);
    let seconds = start.elapsed().as_secs_f64();
    let best = *found.rows.first().context("no place fit this skyline")?;
    let view =
        horizon::view(dem, &cfg.horizon, best.lat, best.lon).context("winner has no ground")?;
    let pose = Pose {
        yaw: best.yaw,
        fov: best.fov,
        roll: best.roll,
        pitch: best.pitch,
    };
    let drawn = outline(
        &view.angles,
        pose,
        f64::from(upload.width),
        f64::from(upload.height),
    );
    let reply = Located {
        seconds,
        best,
        top: found.rows.iter().take(TOP).copied().collect(),
        surface: found
            .rows
            .iter()
            .map(|r| Spot {
                lat: r.lat,
                lon: r.lon,
                loss: r.loss,
            })
            .collect(),
        outline: drawn.iter().map(|&(x, y)| [x, y]).collect(),
        ridge: ridge(&view.distances, best.lat, best.lon, best.yaw, best.fov)
            .iter()
            .map(|&(a, b)| [a, b])
            .collect(),
        skyline: points(&photo.skyline),
        counts: found.counts,
    };
    Ok(json(200, &reply))
}

/// A hand-drawn skyline is a few clicked points; the search wants one per column or so, so
/// straight segments between clicks are filled in every 4 pixels.
fn densify(clicks: &[[f64; 2]]) -> Vec<Pixel> {
    let mut sorted: Vec<[f64; 2]> = clicks.to_vec();
    sorted.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut out = Vec::new();
    for pair in sorted.windows(2) {
        let ([x0, y0], [x1, y1]) = (pair[0], pair[1]);
        let steps = ((x1 - x0) / 4.0).ceil().max(1.0) as usize;
        for k in 0..steps {
            let t = k as f64 / steps as f64;
            out.push(Pixel {
                x: x0 + t * (x1 - x0),
                y: y0 + t * (y1 - y0),
            });
        }
    }
    if let Some(&[x, y]) = sorted.last() {
        out.push(Pixel { x, y });
    }
    out
}

fn points(line: &[Pixel]) -> Vec<[f64; 2]> {
    line.iter().map(|p| [p.x, p.y]).collect()
}

fn header(name: &str, value: &str) -> Option<Header> {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).ok()
}

fn with_type(reply: Reply, content_type: &str) -> Reply {
    match header("Content-Type", content_type) {
        Some(h) => reply.with_header(h),
        None => reply,
    }
}

/// The page is served from another port (the dev server), so every reply allows any origin.
fn cors(mut reply: Reply) -> Reply {
    for (name, value) in [
        ("Access-Control-Allow-Origin", "*"),
        ("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
        ("Access-Control-Allow-Headers", "Content-Type"),
    ] {
        if let Some(h) = header(name, value) {
            reply = reply.with_header(h);
        }
    }
    reply
}

fn json(status: u16, value: &impl Serialize) -> Reply {
    let bytes = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    with_type(
        cors(Response::from_data(bytes).with_status_code(status)),
        "application/json",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn densify_fills_between_clicks_in_x_order() {
        let line = densify(&[[100.0, 50.0], [0.0, 10.0], [40.0, 30.0]]);
        assert_eq!(line.first().map(|p| (p.x, p.y)), Some((0.0, 10.0)));
        assert_eq!(line.last().map(|p| (p.x, p.y)), Some((100.0, 50.0)));
        assert_eq!(
            line.len(),
            10 + 15 + 1,
            "4 px steps: 40 px is 10, 60 px is 15, plus the end"
        );
        assert!(line.windows(2).all(|w| w[0].x <= w[1].x));
    }
}
