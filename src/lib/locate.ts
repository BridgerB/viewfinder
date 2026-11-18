// Pure logic for the /locate page: talking to `sight serve` and reading its answer.

/** A point in image pixels, [x, y]. */
type Point = readonly [number, number];
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

/** The locator runs beside the dev server, on the next port, on the same host. */
const apiBase = (hostname: string) => `http://${hostname}:4373`;

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
const verdict = (best: Place, spread: number): Verdict => {
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
			text: "Unsure: the candidates agree, but the fit is loose. A hand-drawn skyline may help.",
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

/** SVG `points` for a polyline. */
const svgPoints = (points: readonly Point[]) =>
	points.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(" ");

export type { LatLon, Located, Place, Point, Traced, Verdict };
export { apiBase, kmBetween, parseFov, spreadKm, svgPoints, verdict };
