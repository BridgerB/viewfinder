type Coordinate = {
	latitude: number;
	longitude: number;
};

type HorizonPoint = {
	relativeDirection: number;
	elevationAngleDegrees: number;
	distanceKm: number;
};

type Viewpoint = {
	angle: number;
	viewpoint: Coordinate;
	bearingToPeak: number;
	horizon: readonly HorizonPoint[];
};

type Pixel = { x: number; y: number };

type Ray = { azimuth: number; elevation: number };

type Camera = {
	fovH: number;
	centerAzimuth: number;
	roll: number;
	pitch: number;
	imageWidth: number;
	imageHeight: number;
};

type Match = {
	bestAngle: number;
	camera: Camera;
	skyline: readonly Pixel[];
	correlation: readonly number[];
	rmse: readonly number[];
};

type Lens = {
	focalLength: number;
	rollCosine: number;
	rollSine: number;
	centerAzimuth: number;
	pitch: number;
	imageWidth: number;
	imageHeight: number;
};

type Span = { min: number; max: number };

type Profile = {
	photo: string;
	model: string;
	azimuth: Span;
	elevation: Span;
};

type HeatSample = {
	latitude: number;
	longitude: number;
	weight: number;
};

type RawMatch = {
	bestAngle: number;
	fovH: number;
	centerAzimuth: number;
	roll: number;
	pitch: number;
	imageWidth: number;
	imageHeight: number;
	skyline: readonly (readonly number[])[];
	correlation: readonly number[];
	rmse: readonly number[];
};

const toRadians = (degrees: number) => (degrees * Math.PI) / 180;

const toDegrees = (radians: number) => (radians * 180) / Math.PI;

const pixelOf = (pair: readonly number[]): Pixel => {
	const x = pair[0];
	const y = pair[1];
	if (x === undefined || y === undefined) throw new Error('skyline point needs x and y');
	return { x, y };
};

const fromJson = (raw: RawMatch): Match => ({
	bestAngle: raw.bestAngle,
	camera: {
		fovH: raw.fovH,
		centerAzimuth: raw.centerAzimuth,
		roll: raw.roll,
		pitch: raw.pitch,
		imageWidth: raw.imageWidth,
		imageHeight: raw.imageHeight
	},
	skyline: raw.skyline.map(pixelOf),
	correlation: raw.correlation,
	rmse: raw.rmse
});

const lensOf = (camera: Camera): Lens => {
	const roll = toRadians(camera.roll);
	return {
		focalLength: camera.imageWidth / 2 / Math.tan(toRadians(camera.fovH / 2)),
		rollCosine: Math.cos(roll),
		rollSine: Math.sin(roll),
		centerAzimuth: camera.centerAzimuth,
		pitch: camera.pitch,
		imageWidth: camera.imageWidth,
		imageHeight: camera.imageHeight
	};
};

const project = (lens: Lens, pixel: Pixel): Ray => {
	const centeredX = pixel.x - lens.imageWidth / 2;
	const centeredY = lens.imageHeight / 2 - pixel.y;
	const rolledX = centeredX * lens.rollCosine - centeredY * lens.rollSine;
	const rolledY = centeredX * lens.rollSine + centeredY * lens.rollCosine;
	return {
		azimuth: lens.centerAzimuth + toDegrees(Math.atan(rolledX / lens.focalLength)),
		elevation: lens.pitch + toDegrees(Math.atan(rolledY / lens.focalLength))
	};
};

const unproject = (lens: Lens, ray: Ray): Pixel => {
	const rolledX = lens.focalLength * Math.tan(toRadians(ray.azimuth - lens.centerAzimuth));
	const rolledY = lens.focalLength * Math.tan(toRadians(ray.elevation - lens.pitch));
	const centeredX = rolledX * lens.rollCosine + rolledY * lens.rollSine;
	const centeredY = -rolledX * lens.rollSine + rolledY * lens.rollCosine;
	return { x: lens.imageWidth / 2 + centeredX, y: lens.imageHeight / 2 - centeredY };
};

const joinPoints = (points: readonly string[]) => points.join(' ');

const skylinePoints = (skyline: readonly Pixel[]) =>
	joinPoints(skyline.map((pixel) => `${pixel.x},${pixel.y}`));

const isOnFrame = (lens: Lens, pixel: Pixel) =>
	pixel.y >= -400 && pixel.y <= lens.imageHeight + 400;

const horizonPoints = (lens: Lens, horizon: readonly HorizonPoint[]) =>
	joinPoints(
		horizon
			.map((point) =>
				unproject(lens, {
					azimuth: point.relativeDirection,
					elevation: point.elevationAngleDegrees
				})
			)
			.filter((pixel) => isOnFrame(lens, pixel))
			.map((pixel) => `${pixel.x.toFixed(1)},${pixel.y.toFixed(1)}`)
	);

const scorePoints = (correlation: readonly number[]) =>
	joinPoints(
		correlation.map((value, index) => `${index},${((1 - Math.max(0, value)) * 100).toFixed(2)}`)
	);

const spanOf = (values: readonly number[]): Span => ({
	min: Math.min(...values),
	max: Math.max(...values)
});

const padded = (span: Span): Span => {
	const pad = (span.max - span.min) * 0.08;
	return { min: span.min - pad, max: span.max + pad };
};

const elevationSpan = (elevations: readonly number[]): Span => {
	const span = spanOf(elevations);
	if (span.max - span.min >= 8) return padded(span);
	const middle = (span.max + span.min) / 2;
	return padded({ min: middle - 4, max: middle + 4 });
};

const chartPoint = (scales: { azimuth: Span; elevation: Span }, ray: Ray) => {
	const x = ((ray.azimuth - scales.azimuth.min) / (scales.azimuth.max - scales.azimuth.min)) * 100;
	const y =
		((scales.elevation.max - ray.elevation) / (scales.elevation.max - scales.elevation.min)) * 100;
	return `${x.toFixed(2)},${y.toFixed(2)}`;
};

const profileOf = (
	lens: Lens,
	skyline: readonly Pixel[],
	horizon: readonly HorizonPoint[]
): Profile => {
	const photo = skyline.map((pixel) => project(lens, pixel));
	const model = horizon.map((point) => ({
		azimuth: point.relativeDirection,
		elevation: point.elevationAngleDegrees
	}));
	const rays = [...photo, ...model];
	const azimuth = spanOf(rays.map((ray) => ray.azimuth));
	const elevation = elevationSpan(rays.map((ray) => ray.elevation));
	const scales = { azimuth, elevation };
	return {
		photo: joinPoints(photo.map((ray) => chartPoint(scales, ray))),
		model: joinPoints(model.map((ray) => chartPoint(scales, ray))),
		azimuth,
		elevation
	};
};

const clampIndex = (angle: number) => ((Math.round(angle) % 360) + 360) % 360;

const at = (values: readonly number[], angle: number) => values[clampIndex(angle)] ?? 0;

const viewpointAt = (viewpoints: readonly Viewpoint[], angle: number): Viewpoint => {
	const found = viewpoints[clampIndex(angle)];
	if (!found) throw new Error(`unreachable: no viewpoint at ${angle}`);
	return found;
};

const shapeMatch = (match: Match, angle: number) => Math.max(0, at(match.correlation, angle));

const averageError = (match: Match, angle: number) => at(match.rmse, angle);

const angleAt = (ratio: number) => Math.round(Math.min(359, Math.max(0, ratio * 359)));

const coordinateText = (coordinate: Coordinate) =>
	`${coordinate.latitude.toFixed(4)}°N, ${Math.abs(coordinate.longitude).toFixed(4)}°W`;

const signedDegrees = (degrees: number) => `${degrees > 0 ? '+' : ''}${degrees.toFixed(0)}°`;

const percentText = (ratio: number) => `${(ratio * 100).toFixed(1)}%`;

type SearchHit = {
	lat: number;
	lon: number;
	loss: number;
};

const searchHeat = (hits: readonly SearchHit[]) => {
	const best = hits.reduce((chosen, hit) => (hit.loss < chosen.loss ? hit : chosen));
	return {
		best: { latitude: best.lat, longitude: best.lon },
		samples: hits.flatMap((hit) => {
			const weight = Math.exp(-(hit.loss - best.loss) / 0.06);
			if (weight < 0.15) return [];
			return [{ latitude: hit.lat, longitude: hit.lon, weight }];
		})
	};
};

export type { Coordinate, HorizonPoint, Viewpoint, Camera, Match, Lens, Profile, HeatSample, SearchHit };
export {
	fromJson,
	lensOf,
	skylinePoints,
	horizonPoints,
	scorePoints,
	profileOf,
	clampIndex,
	viewpointAt,
	shapeMatch,
	averageError,
	angleAt,
	coordinateText,
	signedDegrees,
	percentText,
	searchHeat
};
