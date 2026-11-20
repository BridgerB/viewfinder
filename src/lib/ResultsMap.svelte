<script lang="ts">
import type { Action } from "svelte/action";
import { fromAction } from "svelte/attachments";
import "leaflet/dist/leaflet.css";
import type { LayerGroup, Map as LeafletMap } from "leaflet";
import { addBasemap, leaflet, loadLeaflet, MAP_INK } from "#lib/basemap.ts";

type Example = {
	id: string;
	name: string;
	truth: [number, number];
	guess: [number, number];
	errorKm: number;
	ridge: [number, number][];
};

let { result }: { result: Example } = $props();

const resultsMap: Action<HTMLDivElement, Example> = (host, first) => {
	let map: LeafletMap | undefined;
	let layer: LayerGroup | undefined;
	let current = first;
	let cancelled = false;

	const draw = (r: Example) => {
		current = r;
		const L = leaflet();
		if (!map || !layer || !L) return;
		layer.clearLayers();
		L.polygon([r.guess, ...r.ridge, r.guess], {
			color: MAP_INK.accent,
			weight: 1.2,
			fillColor: MAP_INK.accent,
			fillOpacity: 0.18,
		}).addTo(layer);
		L.polyline(r.ridge, { color: MAP_INK.ink, weight: 2.5 })
			.bindTooltip("The ridge in the photo")
			.addTo(layer);
		L.polyline([r.truth, r.guess], {
			color: MAP_INK.ink,
			weight: 1.5,
			dashArray: "5 5",
		}).addTo(layer);
		L.circleMarker(r.truth, {
			radius: 6,
			color: "#fff",
			weight: 2.5,
			fillColor: MAP_INK.ink,
			fillOpacity: 1,
		})
			.bindTooltip("Where the photo was taken")
			.addTo(layer);
		L.circleMarker(r.guess, {
			radius: 7,
			color: "#fff",
			weight: 2.5,
			fillColor: MAP_INK.accent,
			fillOpacity: 1,
		})
			.bindTooltip(`Viewfinder’s answer, ${r.errorKm} km off`)
			.addTo(layer);
		const points = [r.truth, ...r.ridge];
		const dLat = Math.max(...points.map((p) => Math.abs(p[0] - r.guess[0])));
		const dLon = Math.max(...points.map((p) => Math.abs(p[1] - r.guess[1])));
		map.invalidateSize();
		map.fitBounds(
			[
				[r.guess[0] - dLat, r.guess[1] - dLon],
				[r.guess[0] + dLat, r.guess[1] + dLon],
			],
			{ padding: [30, 30], maxZoom: 14 },
		);
	};

	const ready = loadLeaflet().then((L) => {
		if (cancelled || !host.isConnected) return;
		map = L.map(host, { scrollWheelZoom: false });
		addBasemap(L, map);
		layer = L.layerGroup().addTo(map);
		draw(current);
	});

	return {
		update(next) {
			draw(next);
		},
		destroy() {
			cancelled = true;
			if (map) map.remove();
			else void ready.then(() => map?.remove());
		},
	};
};
</script>

<div
	class="map"
	{@attach fromAction(resultsMap, () => result)}
	role="img"
	aria-label="Map of {result.name}: the real camera spot, Viewfinder's answer, and the ridge it sees"
></div>

<style>
	.map {
		width: 100%;
		aspect-ratio: 2 / 1;
		min-height: 280px;
		border-radius: 14px;
		overflow: hidden;
		background: var(--soft);
	}

	.map :global(.leaflet-control-attribution) {
		background: rgba(255, 255, 255, 0.8);
		color: var(--faint);
		font: 10px var(--sans);
	}

	.map :global(.leaflet-control-attribution a) {
		border: 0;
	}
</style>
