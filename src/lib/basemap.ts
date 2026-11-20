// Leaflet setup shared by every map. Leaflet needs `window`, so it is imported on demand inside
// an action; `leaflet.heat` needs `L` on `globalThis` before it loads.

import type { Map as LeafletMap } from "leaflet";

type Leaflet = typeof import("leaflet");

/** Leaflet once loaded, for code that runs after `loadLeaflet` has finished. */
let loaded: Leaflet | undefined;
const leaflet = () => loaded;

/** Map colours, matching the site's tokens in app.css. */
const MAP_INK = { ink: "#222", accent: "#c8553d" } as const;

/** Leaflet, with `leaflet.heat` added when `heat` is set. */
const loadLeaflet = async ({ heat = false } = {}): Promise<Leaflet> => {
	const L = (await import("leaflet")).default;
	loaded = L;
	(globalThis as typeof globalThis & { L: Leaflet }).L = L;
	if (heat) await import("leaflet.heat");
	return L;
};

/** OpenStreetMap tiles; app.css mutes them so the pin, wedge and heat carry the colour. */
const addBasemap = (L: Leaflet, map: LeafletMap) =>
	L.tileLayer("https://tile.openstreetmap.org/{z}/{x}/{y}.png", {
		maxZoom: 19,
		className: "muted-tiles",
		attribution:
			'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>',
	}).addTo(map);

export { addBasemap, leaflet, loadLeaflet, MAP_INK };
