<script lang="ts" module>
/** Where a photo was really taken, as marked by hand on /dev. Yaw is a compass bearing. */
export type Truth = { lat: number; lon: number; yaw: number };
/** A camera the locator answered with. */
export type Camera = { lat: number; lon: number; yaw: number; fov: number };

const M_PER_DEG = 111_320;

/** The point `km` from (lat, lon) along compass bearing `yaw`, on a local flat frame. */
export const ahead = (
	lat: number,
	lon: number,
	yaw: number,
	km: number,
): [number, number] => {
	const r = (yaw * Math.PI) / 180;
	const m = km * 1000;
	return [
		lat + (m * Math.cos(r)) / M_PER_DEG,
		lon + (m * Math.sin(r)) / (M_PER_DEG * Math.cos((lat * Math.PI) / 180)),
	];
};

/** Kilometres and compass bearing from a to b, on a local flat frame. */
export const offset = (
	a: { lat: number; lon: number },
	b: { lat: number; lon: number },
) => {
	const dy = (b.lat - a.lat) * M_PER_DEG;
	const dx = (b.lon - a.lon) * M_PER_DEG * Math.cos((a.lat * Math.PI) / 180);
	return {
		km: Math.hypot(dx, dy) / 1000,
		yaw: ((Math.atan2(dx, dy) * 180) / Math.PI + 360) % 360,
	};
};
</script>

<script lang="ts">
import type { Attachment } from "svelte/attachments";
import "leaflet/dist/leaflet.css";
import type { LayerGroup, Map as LeafletMap } from "leaflet";
import { addBasemap, leaflet, loadLeaflet, MAP_INK } from "#lib/basemap.ts";

let {
	answer,
	rerun,
	truth,
	fov,
	onchange,
}: {
	/** The live share's answer. */
	answer: Camera;
	/** The local rerun's answer, when there is one. */
	rerun?: Camera;
	truth: Truth | undefined;
	/** Lens used to draw the true view, degrees. */
	fov: number;
	onchange: (t: Truth) => void;
} = $props();

/** How far the view wedges reach, km: the handle that turns the true camera sits at its tip. */
let reach = $state(8);
let map = $state<LeafletMap>();
let layer: LayerGroup | undefined;

const picker: Attachment<HTMLDivElement> = (host) => {
	let cancelled = false;
	let made: LeafletMap | undefined;
	void loadLeaflet().then((L) => {
		if (cancelled || !host.isConnected) return;
		const start = truth ?? answer;
		made = L.map(host, { scrollWheelZoom: true }).setView(
			[start.lat, start.lon],
			11,
		);
		addBasemap(L, made);
		layer = L.layerGroup().addTo(made);
		// A click puts the true camera there, facing where it faced before (or the answer's way).
		made.on("click", (e) =>
			onchange({
				lat: e.latlng.lat,
				lon: e.latlng.lng,
				yaw: truth?.yaw ?? answer.yaw,
			}),
		);
		map = made;
	});
	return () => {
		cancelled = true;
		made?.remove();
		map = undefined;
	};
};

$effect(() => {
	const L = leaflet();
	if (!map || !layer || !L) return;
	layer.clearLayers();

	const wedge = (c: Camera | Truth, width: number, color: string, dash?: string) => {
		const half = width / 2;
		const arc = Array.from({ length: 13 }, (_, i) =>
			ahead(c.lat, c.lon, c.yaw - half + (i * width) / 12, reach),
		);
		L.polygon([[c.lat, c.lon], ...arc], {
			color,
			weight: 1.5,
			dashArray: dash,
			fillColor: color,
			fillOpacity: 0.08,
			interactive: false,
		}).addTo(layer as LayerGroup);
	};
	const dot = (c: { lat: number; lon: number }, color: string, tip: string) =>
		L.circleMarker([c.lat, c.lon], {
			radius: 6,
			color: "#fff",
			weight: 2,
			fillColor: color,
			fillOpacity: 1,
		})
			.bindTooltip(tip)
			.addTo(layer as LayerGroup);

	wedge(answer, answer.fov, MAP_INK.accent);
	dot(answer, MAP_INK.accent, "Live answer");
	if (rerun) {
		wedge(rerun, rerun.fov, "#2b7bb9", "4 4");
		dot(rerun, "#2b7bb9", "Local rerun");
	}
	if (truth) {
		wedge(truth, fov, MAP_INK.ink);
		const pin = (at: [number, number], tip: string, size: number) =>
			L.marker(at, {
				draggable: true,
				icon: L.divIcon({
					className: "truth-handle",
					iconSize: [size, size],
					html: "",
				}),
				title: tip,
			}).addTo(layer as LayerGroup);
		pin([truth.lat, truth.lon], "Where it was taken: drag to move", 18).on(
			"dragend",
			(e) => {
				const p = e.target.getLatLng();
				onchange({ lat: p.lat, lon: p.lng, yaw: truth.yaw });
			},
		);
		pin(ahead(truth.lat, truth.lon, truth.yaw, reach), "Drag to turn the camera", 14).on(
			"dragend",
			(e) => {
				const p = e.target.getLatLng();
				const o = offset(truth, { lat: p.lat, lon: p.lng });
				reach = Math.min(40, Math.max(1, o.km));
				onchange({ ...truth, yaw: Math.round(o.yaw) });
			},
		);
	}
});
</script>

<div class="map" {@attach picker} role="application" aria-label="Mark where the photo was taken"></div>

<style>
	.map {
		width: 100%;
		aspect-ratio: 4 / 3;
		min-height: 260px;
		border-radius: 14px;
		overflow: hidden;
		background: var(--soft);
		cursor: crosshair;
	}

	.map :global(.truth-handle) {
		border: 3px solid #fff;
		border-radius: 50%;
		background: #222;
		box-shadow: 0 1px 4px rgb(0 0 0 / 0.5);
		cursor: grab;
	}
</style>
