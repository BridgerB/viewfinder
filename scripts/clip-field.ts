// Keeps the places of a horizon field (HZN2) that lie inside any polygon of a GeoJSON file, or
// within BUFFER metres of one, so a camera standing on the line still has a place near it.
//
//   node scripts/clip-field.ts AREAS.geojson IN.hzn OUT.hzn [BUFFER_M=1000] [--outside S,N,W,E]
//
// --outside also drops every place inside that box, for adding new places around an area that
// already has its own field.
//
// HZN2: 12-byte header (magic "HZN2", i32 count, i32 azimuths), then one fixed-size record per
// place starting with f64 lat, f64 lon (see brain/locator/data/horizon-field.md).
import { readFileSync, writeFileSync } from "node:fs";

type Ring = [number, number][];
type Geometry =
	| { type: "Polygon"; coordinates: Ring[] }
	| { type: "MultiPolygon"; coordinates: Ring[][] };

const args = process.argv.slice(2);
const outsideAt = args.indexOf("--outside");
const outside =
	outsideAt >= 0 ? args.splice(outsideAt, 2)[1].split(",").map(Number) : null;
const [areasPath, inPath, outPath, bufferText = "1000"] = args;
if (!areasPath || !inPath || !outPath) {
	console.error(
		"usage: node scripts/clip-field.ts AREAS.geojson IN.hzn OUT.hzn [BUFFER_M]",
	);
	process.exit(1);
}
const buffer = Number(bufferText);

const areas = JSON.parse(readFileSync(areasPath, "utf8")) as {
	features: { geometry: Geometry }[];
};
/** Every polygon as its rings (outer first, then holes), [lon, lat] pairs. */
const polygons: Ring[][] = areas.features.flatMap(({ geometry: g }) =>
	g.type === "Polygon" ? [g.coordinates] : g.coordinates,
);

const inside = (ring: Ring, lon: number, lat: number) => {
	let hit = false;
	for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
		const [xi, yi] = ring[i];
		const [xj, yj] = ring[j];
		if (
			yi > lat !== yj > lat &&
			lon < ((xj - xi) * (lat - yi)) / (yj - yi) + xi
		)
			hit = !hit;
	}
	return hit;
};

/** Metres from a point to the nearest edge of a ring, in a local flat frame. */
const distance = (ring: Ring, lon: number, lat: number) => {
	const kx = 111_320 * Math.cos((lat * Math.PI) / 180);
	const ky = 111_320;
	let best = Number.POSITIVE_INFINITY;
	for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
		const ax = (ring[j][0] - lon) * kx;
		const ay = (ring[j][1] - lat) * ky;
		const bx = (ring[i][0] - lon) * kx;
		const by = (ring[i][1] - lat) * ky;
		const dx = bx - ax;
		const dy = by - ay;
		const t = Math.max(
			0,
			Math.min(1, -(ax * dx + ay * dy) / (dx * dx + dy * dy || 1)),
		);
		best = Math.min(best, Math.hypot(ax + t * dx, ay + t * dy));
	}
	return best;
};

const inBox = (lat: number, lon: number) =>
	outside !== null &&
	lat >= outside[0] &&
	lat <= outside[1] &&
	lon >= outside[2] &&
	lon <= outside[3];

const keep = (lat: number, lon: number) =>
	!inBox(lat, lon) &&
	polygons.some(
		([outer, ...holes]) =>
			(inside(outer, lon, lat) && !holes.some((h) => inside(h, lon, lat))) ||
			distance(outer, lon, lat) <= buffer,
	);

const data = readFileSync(inPath);
if (data.toString("latin1", 0, 4) !== "HZN2")
	throw new Error(`${inPath} is not an HZN2 field`);
const count = data.readInt32LE(4);
const azimuths = data.readInt32LE(8);
const record = 8 + 8 + 4 + azimuths * 2 + azimuths * 4;
if (data.length !== 12 + count * record)
	throw new Error(
		`${inPath}: ${data.length} bytes, expected ${12 + count * record}`,
	);

const kept: Buffer[] = [];
for (let i = 0; i < count; i++) {
	const at = 12 + i * record;
	if (keep(data.readDoubleLE(at), data.readDoubleLE(at + 8)))
		kept.push(data.subarray(at, at + record));
}
const header = Buffer.alloc(12);
header.write("HZN2", 0, "latin1");
header.writeInt32LE(kept.length, 4);
header.writeInt32LE(azimuths, 8);
writeFileSync(outPath, Buffer.concat([header, ...kept]));
console.log(
	`kept ${kept.length} of ${count} places within ${buffer} m of the areas → ${outPath}`,
);
