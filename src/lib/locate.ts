// Pure logic for the /locate page: talking to `sight serve` and reading its answer.

/** A point in image pixels, [x, y]. */
type Point = readonly [number, number];
/** A box in image pixels: [x0, y0, x1, y1] with x0 < x1 and y0 < y1. */
type Box = [number, number, number, number];
/** A place on the map, [lat, lon]. */
type LatLon = readonly [number, number];

/** What `POST /trace` returns for an uploaded photo. */
type Traced = {
	id: number;
	width: number;
	height: number;
	/** Lens width in degrees from EXIF, or null when the photo does not record it. */
	fov: number | null;
	skyline: Point[];
};

/** One ranked camera place, as the locator reports it. */
type Place = {
	lat: number;
	lon: number;
	elev: number;
	yaw: number;
	fov: number;
	roll: number;
	pitch: number;
	rmse: number;
	corr: number;
	loss: number;
};

/** What `POST /locate` returns. */
type Located = {
	seconds: number;
	best: Place;
	top: Place[];
	surface: { lat: number; lon: number; loss: number }[];
	outline: Point[];
	ridge: LatLon[];
	skyline: Point[];
	counts: number[];
};

/** The locator, same-origin: the site forwards /api/sight/* to it (see the route's comment). */
const API = "/api/sight";

const toRadians = (degrees: number) => (degrees * Math.PI) / 180;

/** Great-circle distance in km. */
const kmBetween = (a: LatLon, b: LatLon) => {
	const dLat = toRadians(b[0] - a[0]);
	const dLon = toRadians(b[1] - a[1]);
	const h =
		Math.sin(dLat / 2) ** 2 +
		Math.cos(toRadians(a[0])) *
			Math.cos(toRadians(b[0])) *
			Math.sin(dLon / 2) ** 2;
	return 2 * 6371 * Math.asin(Math.sqrt(h));
};

/** How far the next-best candidates (up to `count`) sit from the best one, km. */
const spreadKm = (top: readonly Place[], count = 5) => {
	const [best, ...rest] = top;
	if (!best) return 0;
	return Math.max(
		0,
		...rest
			.slice(0, count - 1)
			.map((p) => kmBetween([best.lat, best.lon], [p.lat, p.lon])),
	);
};

type Verdict = { level: "good" | "unsure" | "poor"; text: string };

/**
 * How much to trust an answer. The loss alone is not enough: a smooth skyline fits many places
 * well, so the best candidates must also agree with each other. Solved test photos score
 * 0.1–0.3 with their top candidates within about 1 km.
 */
/**
 * How much shape a skyline has: the spread of its points around their best straight line, as a
 * share of the photo's height. A real ridge is about 1–2%; a trace that slid along a smooth
 * line (haze, a cloud bank) is about 0.1% and fits many places with a deceptively low loss.
 */
const shapeOf = (line: readonly Point[], height: number) => {
	const n = line.length;
	if (n < 2 || height <= 0) return 0;
	const mx = line.reduce((a, p) => a + p[0], 0) / n;
	const my = line.reduce((a, p) => a + p[1], 0) / n;
	const sxx = line.reduce((a, p) => a + (p[0] - mx) ** 2, 0);
	const sxy = line.reduce((a, p) => a + (p[0] - mx) * (p[1] - my), 0);
	const slope = sxx ? sxy / sxx : 0;
	const rss = line.reduce(
		(a, p) => a + (p[1] - my - slope * (p[0] - mx)) ** 2,
		0,
	);
	return Math.sqrt(rss / n) / height;
};

/** Below this `shapeOf`, a skyline is too close to a straight line to place a camera. */
const MIN_SHAPE = 0.004;

const verdict = (best: Place, spread: number, shape = 1): Verdict => {
	if (shape < MIN_SHAPE)
		return {
			level: "unsure",
			text: "Unsure: the traced skyline is almost a straight line, so it fits many places. Check that the orange line follows the ridge.",
		};
	if (best.loss > 0.6)
		return {
			level: "poor",
			text: "Poor fit: the skyline matches nowhere well. Check that the traced line follows the ridge.",
		};
	if (spread > 3)
		return {
			level: "unsure",
			text: `Unsure: the best candidates are up to ${spread.toFixed(1)} km apart, so this skyline fits several places.`,
		};
	if (best.loss > 0.3)
		return {
			level: "unsure",
			text: "Unsure: the candidates agree, but the fit is loose. Check that the orange line follows the ridge.",
		};
	return {
		level: "good",
		text: `Good fit, and the best candidates agree within ${spread.toFixed(1)} km.`,
	};
};

/** Lens width from the input box: a number in 10–130°, or null to try every lens. */
const parseFov = (text: string): number | null => {
	const value = Number.parseFloat(text);
	return Number.isFinite(value) && value >= 10 && value <= 130 ? value : null;
};

/**
 * A box around a skyline, as a starting point to adjust: its full width, with room above for
 * sky and a little below, clamped to the photo.
 */
const fitBox = (line: readonly Point[], width: number, height: number): Box => {
	if (line.length === 0) return [0, 0, width, height];
	const xs = line.map((p) => p[0]);
	const ys = line.map((p) => p[1]);
	const [minX, maxX, minY, maxY] = [
		Math.min(...xs),
		Math.max(...xs),
		Math.min(...ys),
		Math.max(...ys),
	];
	const relief = maxY - minY;
	return [
		Math.max(0, minX - 0.01 * width),
		Math.max(0, minY - Math.max(0.06 * height, 0.6 * relief)),
		Math.min(width, maxX + 0.01 * width),
		Math.min(height, maxY + Math.max(0.05 * height, 0.4 * relief)),
	];
};

/** What a pointer grabbed on a box: an edge or corner (compass point), the inside, or nothing. */
type Grab = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw" | "move";

/** The part of `box` under `p`, with `reach` image pixels of slack around edges. */
const grabAt = (box: Box, p: Point, reach: number): Grab | null => {
	const [x0, y0, x1, y1] = box;
	const [x, y] = p;
	if (x < x0 - reach || x > x1 + reach || y < y0 - reach || y > y1 + reach)
		return null;
	const v =
		Math.abs(y - y0) <= reach ? "n" : Math.abs(y - y1) <= reach ? "s" : "";
	const h =
		Math.abs(x - x0) <= reach ? "w" : Math.abs(x - x1) <= reach ? "e" : "";
	return ((v + h) as Grab) || "move";
};

/** `start` with `grab` dragged from `from` to `to`, kept inside the photo and at least 32 px. */
const dragBox = (
	start: Box,
	grab: Grab,
	from: Point,
	to: Point,
	width: number,
	height: number,
): Box => {
	const dx = to[0] - from[0];
	const dy = to[1] - from[1];
	let [x0, y0, x1, y1] = start;
	if (grab === "move") {
		const mx = Math.min(Math.max(dx, -x0), width - x1);
		const my = Math.min(Math.max(dy, -y0), height - y1);
		return [x0 + mx, y0 + my, x1 + mx, y1 + my];
	}
	const min = 32;
	if (grab.includes("w")) x0 = Math.min(Math.max(0, x0 + dx), x1 - min);
	if (grab.includes("e")) x1 = Math.max(Math.min(width, x1 + dx), x0 + min);
	if (grab.includes("n")) y0 = Math.min(Math.max(0, y0 + dy), y1 - min);
	if (grab.includes("s")) y1 = Math.max(Math.min(height, y1 + dy), y0 + min);
	return [x0, y0, x1, y1];
};

/** The cursor that says what dragging here does. */
const grabCursor = (grab: Grab | null) =>
	grab === null
		? "crosshair"
		: grab === "move"
			? "move"
			: grab === "n" || grab === "s"
				? "ns-resize"
				: grab === "e" || grab === "w"
					? "ew-resize"
					: grab === "nw" || grab === "se"
						? "nwse-resize"
						: "nesw-resize";

/** Whether `id` looks like a share id (a random UUID), before it reaches R2 or D1. */
const isShareId = (id: string) =>
	/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(id);

/** SVG `points` for a polyline. */
const svgPoints = (points: readonly Point[]) =>
	points.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(" ");

export type { Box, Grab, LatLon, Located, Place, Point, Traced, Verdict };
export {
	API,
	dragBox,
	fitBox,
	grabAt,
	grabCursor,
	isShareId,
	kmBetween,
	MIN_SHAPE,
	parseFov,
	shapeOf,
	spreadKm,
	svgPoints,
	verdict,
};
