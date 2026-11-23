---
type: area
status: verified
verifiedAgainst: 01608dc, 2026-11-20
sourceRefs:
  - sight/crates/sight-core/src/skymap.rs#pub fn trace_skymap(img: &Image, map: &SkyMap, region: Region) -> Vec<Pixel> {
  - sight/crates/sight-core/src/skymap.rs#const JUMP: f64 = 6.0;
  - sight/crates/sight-core/src/skymap.rs#const OVERHANG: f64 = 0.1;
  - sight/crates/sight-core/src/skymap.rs#const LEANS: f64 = 0.5;
  - sight/crates/sight-core/src/skymap.rs#pub fn drop_structures(line: &[Pixel], height: usize) -> Vec<Pixel> {
  - sight/crates/sight-io/src/skyseg.rs#const SIZE: usize = 320;
  - cloud/sight/Dockerfile#COPY data/models/skyseg.onnx /data/
---

# Sky model tracer

With `--sky data/models/skyseg.onnx` (`serve`, `bench`, `sky-trace`), the skyline comes from a sky-segmentation network instead of the colour rules ([[skyline-trace]]).

## Why

The colour rules decide sky pixel by pixel. White snow reads as cloud, so on snowy peaks the line dropped to the dark rock below; dark cloud reads as rock; a pole breaks the line. On the 12 photos people uploaded to the live site most traces were wrong in one of these ways. The literature agrees that automatic sky segmentation is the hard part: the ETH Alps work needed a person to correct it on 40–50% of photos, and fully convolutional networks beat hand rules on mountain photos (Ahmad et al., arXiv 1805.08105).

## How

1. **Network.** U²-Net from Sky-Segmentation-and-Post-processing, as `skyseg.onnx` (Hugging Face `JianyuanWang/skyseg`, MIT, 176 MB). The photo is averaged down to 320 × 320, normalised with the ImageNet mean and standard deviation, and run on the CPU by tract (pure Rust, so the x86-64 container needs no native library). The output is stretched to 0–1, higher for sky. About 0.6 s on the laptop.
2. **Path.** Dynamic programming finds one continuous line across the photo (Lie et al.'s shortest-path skyline): each column pays for ground it leaves above the line and sky it leaves below; a jump of the full height between neighbouring columns costs `JUMP` = 6, against a column cost of at most 1, so a cloud or a pole a few columns wide is not worth leaving the ridge for. Up to 1024 columns × 640 rows, under 0.01 s.
   - **Overhangs.** Ground hanging from the top of the frame (branches of a tree beside the camera) costs `OVERHANG` = 0.1 per row instead of 1, while sky inside that hanging stretch still costs 1. Paying full price, a canopy over a third of the frame made "all ground" the cheapest answer in its columns, and the golf course photo lost Mount Olympus under its tree (0.3 still lost it). Only discounting ground near the line instead (a fixed window) let the line drop under a lake's reflection at the bottom of the frame, because nothing above the window counted.
3. **Unsure columns dropped.** A column is kept if the map is ≥ 0.85 just above the line and ≤ 0.5 just below, or if the map only leans sky above (≥ `LEANS` = 0.5) and the photo itself steps down from sky to ground there by at least twice the box tracer's `MIN_STEP`: grey cloud over a dark summit is half-sure in the map but a hard edge in the photo.
4. **Snap.** Each point moves to the photo's sharpest drop in brightness + blueness within one map cell, for pixel precision.
5. **Buildings dropped.** `drop_structures` removes stretches that stand on walls (a jump ≥ 2% of the height between neighbouring points) above their neighbours, are under 20% of the width, and are flatter than the walls they stand on: roofs and towers in front of the ridge. Capitol Hill under cloud went from 57.5 km to 2.56 km with it.

A box from the upload page traces the same map inside the box. If the map finds fewer than 24 points the colour rules are used instead.

## Results

`bench --no-guides` traces every photo itself instead of using the hand-traced skylines:

| tracer | within 1 km | within 3 km | misses |
|---|---|---|---|
| colour rules | 2 of 8 | 2 | 7–109 km |
| sky model | 5 of 8 | 7 | library 122 km |

With the hand-traced skylines nothing changes. The library photo's trace sits 10–15 px above the hand-traced line where low white cloud touches the snowy peaks; that is enough to lose it. Telephoto photos (common on the web: a downtown skyline with the range compressed behind it) need lenses under 40°, and adding 20–35° to the search broke Capitol Hill (0.6 → 132 km), so the lens range is unchanged ([[concepts/lens-distance-tradeoff]]).

Of the 12 photos uploaded to the live site, 5 land right with these traces (marked by hand on `/dev`). Five more are telephoto shots (an 11–14° lens fits the true spot), one was taken from high above the ground, and one (the golf course) has cloud resting on the summits over half the frame: the trace follows the cloud base there, and the search lands 35 km off; with only the clear half of the trace it lands 1.6 km from the true spot. Neither the photo's contrast at the line nor its brightness step tells that cloud base from a real hazy ridge: on other photos real ridges are as soft (snow against grey sky, hazy Capitol Hill).
