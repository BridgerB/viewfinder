// Names for places and directions, offline: no geocoding service sees anyone's coordinates.

import { kmBetween, type LatLon } from "#lib/locate.ts";

/** Towns and resorts in and around Salt Lake, Utah and Davis counties, [name, lat, lon]. */
const TOWNS: readonly (readonly [string, number, number])[] = [
	["Salt Lake City", 40.7608, -111.891],
	["South Salt Lake", 40.7188, -111.8883],
	["Millcreek", 40.6869, -111.8755],
	["Holladay", 40.6688, -111.8247],
	["Murray", 40.6669, -111.8879],
	["Cottonwood Heights", 40.6197, -111.8102],
	["Midvale", 40.6111, -111.8999],
	["Taylorsville", 40.6677, -111.9388],
	["West Valley City", 40.6916, -112.0011],
	["West Jordan", 40.6097, -111.9391],
	["South Jordan", 40.5622, -111.9297],
	["Sandy", 40.5649, -111.8389],
	["Draper", 40.5247, -111.8638],
	["Riverton", 40.5219, -111.9391],
	["Bluffdale", 40.4897, -111.9388],
	["Herriman", 40.5141, -112.033],
	["Alta", 40.5884, -111.6386],
	["Snowbird", 40.581, -111.6566],
	["Brighton", 40.603, -111.5833],
	["Park City", 40.6461, -111.498],
	["Midway", 40.5122, -111.4744],
	["Heber City", 40.5069, -111.4132],
	["Kamas", 40.643, -111.2804],
	["Lehi", 40.3916, -111.8508],
	["Alpine", 40.4533, -111.778],
	["Highland", 40.425, -111.7949],
	["Cedar Hills", 40.4141, -111.7585],
	["American Fork", 40.3769, -111.7958],
	["Pleasant Grove", 40.3641, -111.7385],
	["Saratoga Springs", 40.3491, -111.9047],
	["Eagle Mountain", 40.3141, -112.0069],
	["Lindon", 40.3433, -111.7208],
	["Vineyard", 40.2972, -111.7466],
	["Orem", 40.2969, -111.6946],
	["Sundance", 40.3925, -111.588],
	["Provo", 40.2338, -111.6585],
	["Springville", 40.1652, -111.6107],
	["Mapleton", 40.1302, -111.5785],
	["Bountiful", 40.8894, -111.8808],
	["North Salt Lake", 40.8486, -111.9069],
	["Woods Cross", 40.8716, -111.8922],
	["Centerville", 40.918, -111.8722],
	["Farmington", 40.9805, -111.8874],
	["Kaysville", 41.0352, -111.9386],
	["Layton", 41.0602, -111.9711],
	["Syracuse", 41.0894, -112.0647],
	["Clearfield", 41.1108, -112.0261],
	["Antelope Island", 40.9575, -112.2147],
	["Magna", 40.7091, -112.1016],
	["Copperton", 40.5644, -112.0938],
	["Spanish Fork", 40.1149, -111.6549],
	["Salem", 40.053, -111.6735],
	["Payson", 40.0444, -111.7321],
	["Santaquin", 39.9755, -111.7849],
	["Lake Shore", 40.1197, -111.7499],
	["Goshen", 39.9516, -111.9005],
];

/** A name for where a point is: the town it is in, near, or just the region. */
const placeName = (at: LatLon) => {
	let best = TOWNS[0];
	let bestKm = Number.POSITIVE_INFINITY;
	for (const town of TOWNS) {
		const km = kmBetween(at, [town[1], town[2]]);
		if (km < bestKm) [best, bestKm] = [town, km];
	}
	if (bestKm <= 3) return `${best[0]}, Utah`;
	if (bestKm <= 15) return `Near ${best[0]}, Utah`;
	return "Wasatch Front, Utah";
};

const POINTS = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];

/** Compass point for a bearing in degrees: 128 → "SE". */
const compass = (degrees: number) =>
	POINTS[Math.round((((degrees % 360) + 360) % 360) / 45) % 8];

/** Latitude and longitude written out: "40.56987° N, 111.81507° W". */
const latLonText = ([lat, lon]: LatLon) =>
	`${Math.abs(lat).toFixed(5)}° ${lat >= 0 ? "N" : "S"}, ${Math.abs(lon).toFixed(5)}° ${lon >= 0 ? "E" : "W"}`;

export { compass, latLonText, placeName };
