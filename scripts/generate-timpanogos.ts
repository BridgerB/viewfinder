import { loadElevationData } from '@bridgerb/horizon';
import { writeFileSync } from 'fs';

interface Coordinate {
	latitude: number;
	longitude: number;
}

interface HorizonPoint {
	relativeDirection: number;
	elevationAngleDegrees: number;
	distanceKm: number;
}

interface RawHorizonPoint {
	direction: number;
	elevationAngleDegrees: number;
	distanceKm: number;
}

interface Viewpoint {
	angle: number;
	viewpoint: Coordinate;
	bearingToPeak: number;
	horizon: HorizonPoint[];
}

interface DirectionRange {
	start: number;
	end: number;
}

interface ElevationData {
	calculateHorizon: (
		lat: number,
		lon: number,
		startDir: number,
		endDir: number
	) => RawHorizonPoint[];
}

const PEAK: Coordinate = { latitude: 40.3908, longitude: -111.6458 };
const DISTANCE_KM = 8.919;
const TIF_PATH = 'data/n41w112_30m.tif';
const OUTPUT_PATH_JSON = 'static/timpanogos.json';
const EARTH_RADIUS_KM = 6371;
const HALF_FOV_DEGREES = 45;

const toRadians = (degrees: number): number => (degrees * Math.PI) / 180;

const toDegrees = (radians: number): number => (radians * 180) / Math.PI;

const normalizeAngle = (angle: number): number => ((angle % 360) + 360) % 360;

const normalizeRelativeDirection = (direction: number): number => {
	if (direction > 180) return direction - 360;
	if (direction < -180) return direction + 360;
	return direction;
};

const calculateDestinationPoint = (
	origin: Coordinate,
	distanceKm: number,
	bearingDegrees: number
): Coordinate => {
	const angularDistance = distanceKm / EARTH_RADIUS_KM;
	const bearingRadians = toRadians(bearingDegrees);
	const originLatitudeRadians = toRadians(origin.latitude);
	const originLongitudeRadians = toRadians(origin.longitude);

	const destinationLatitudeRadians = Math.asin(
		Math.sin(originLatitudeRadians) * Math.cos(angularDistance) +
			Math.cos(originLatitudeRadians) * Math.sin(angularDistance) * Math.cos(bearingRadians)
	);

	const destinationLongitudeRadians =
		originLongitudeRadians +
		Math.atan2(
			Math.sin(bearingRadians) * Math.sin(angularDistance) * Math.cos(originLatitudeRadians),
			Math.cos(angularDistance) -
				Math.sin(originLatitudeRadians) * Math.sin(destinationLatitudeRadians)
		);

	return {
		latitude: toDegrees(destinationLatitudeRadians),
		longitude: toDegrees(destinationLongitudeRadians)
	};
};

const calculateBearing = (from: Coordinate, to: Coordinate): number => {
	const deltaLongitudeRadians = toRadians(to.longitude - from.longitude);
	const fromLatitudeRadians = toRadians(from.latitude);
	const toLatitudeRadians = toRadians(to.latitude);

	const eastwardComponent = Math.sin(deltaLongitudeRadians) * Math.cos(toLatitudeRadians);
	const northwardComponent =
		Math.cos(fromLatitudeRadians) * Math.sin(toLatitudeRadians) -
		Math.sin(fromLatitudeRadians) * Math.cos(toLatitudeRadians) * Math.cos(deltaLongitudeRadians);

	return normalizeAngle(toDegrees(Math.atan2(eastwardComponent, northwardComponent)));
};

const calculateDirectionRange = (bearingToPeak: number): DirectionRange => ({
	start: Math.floor(normalizeAngle(bearingToPeak - HALF_FOV_DEGREES)),
	end: Math.floor(normalizeAngle(bearingToPeak + HALF_FOV_DEGREES))
});

const transformToRelativeDirection = (
	point: RawHorizonPoint,
	bearingToPeak: number
): HorizonPoint => ({
	relativeDirection: normalizeRelativeDirection(point.direction - Math.round(bearingToPeak)),
	elevationAngleDegrees: point.elevationAngleDegrees,
	distanceKm: point.distanceKm
});

const calculateHorizonForViewpoint = (
	elevation: ElevationData,
	viewpoint: Coordinate,
	bearingToPeak: number
): HorizonPoint[] => {
	const range = calculateDirectionRange(bearingToPeak);
	const rangeWrapsAroundNorth = range.start > range.end;

	const rawPoints = rangeWrapsAroundNorth
		? [
				...elevation.calculateHorizon(viewpoint.latitude, viewpoint.longitude, range.start, 359),
				...elevation.calculateHorizon(viewpoint.latitude, viewpoint.longitude, 0, range.end)
			]
		: elevation.calculateHorizon(viewpoint.latitude, viewpoint.longitude, range.start, range.end);

	return rawPoints
		.map((point) => transformToRelativeDirection(point, bearingToPeak))
		.sort((pointA, pointB) => pointA.relativeDirection - pointB.relativeDirection);
};

const generateViewpoint = (elevation: ElevationData, angle: number): Viewpoint => {
	const viewpoint = calculateDestinationPoint(PEAK, DISTANCE_KM, angle);
	const bearingToPeak = calculateBearing(viewpoint, PEAK);
	const horizon = calculateHorizonForViewpoint(elevation, viewpoint, bearingToPeak);

	return { angle, viewpoint, bearingToPeak, horizon };
};

async function main(): Promise<void> {
	console.log('Loading elevation data...');
	const elevation = await loadElevationData(TIF_PATH);
	console.log('Generating 360 viewpoints...');

	const viewpoints: Viewpoint[] = [];

	for (let angle = 0; angle < 360; angle++) {
		viewpoints.push(generateViewpoint(elevation, angle));
		if (angle % 60 === 0) console.log(`Generated ${angle}/360`);
	}

	const json = JSON.stringify(viewpoints);
	console.log(`JSON size: ${(json.length / 1024).toFixed(0)}KB`);

	writeFileSync(OUTPUT_PATH_JSON, json);
	console.log(`Saved to ${OUTPUT_PATH_JSON}`);
}

main().catch(console.error);
