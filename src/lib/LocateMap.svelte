<script lang="ts">
import type { LayerGroup, Map as LeafletMap } from "leaflet";
import type { Action } from "svelte/action";
import { fromAction } from "svelte/attachments";
import "leaflet/dist/leaflet.css";
import { addBasemap, leaflet, loadLeaflet, MAP_INK } from "#lib/basemap.ts";
import { type Located, searchHeat } from "#lib/locate.ts";

let {
	result,
	heat = "#c8553d",
	ratio = "2 / 1",
}: {
	result: Located;
	/** The heat's colour: the verdict's colour, so a weak answer looks weak on the map too. */
	heat?: string;
	ratio?: string;
} = $props();

type HeatLayer = (
	points: Array<[number, number, number]>,
	options: Record<string, unknown>,
) => { addTo: (layer: LayerGroup) => unknown };

const locateMap: Action<HTMLDivElement, { result: Located; heat: string }> = (
	host,
	first,
) => {
	let map: LeafletMap | undefined;
	let layer: LayerGroup | undefined;
	let current = first;
	let cancelled = false;

	const draw = ({ result: r, heat: color }: typeof first) => {
		current = { result: r, heat: color };
		const L = leaflet();
		if (!map || !layer || !L) return;
		layer.clearLayers();
		const guess: [number, number] = [r.best.lat, r.best.lon];
		(L as typeof L & { heatLayer: HeatLayer })
			.heatLayer(searchHeat(r.surface), {
				radius: 26,
				blur: 22,
				minOpacity: 0.25,
				max: 1,
				gradient: { 0.3: `${color}33`, 0.7: `${color}99`, 1: color },
			})
			.addTo(layer);
		const ridge = r.ridge.map(([lat, lon]) => [lat, lon] as [number, number]);
		if (ridge.length > 1) {
			L.polygon([guess, ...ridge, guess], {
				color: MAP_INK.accent,
				weight: 1.2,
				fillColor: MAP_INK.accent,
				fillOpacity: 0.18,
			}).addTo(layer);
			L.polyline(ridge, { color: MAP_INK.ink, weight: 2.5 })
				.bindTooltip("The ridge in your photo")
				.addTo(layer);
		}
		for (const [i, p] of r.top.slice(1, 5).entries()) {
			L.circleMarker([p.lat, p.lon], {
				radius: 4,
				color: MAP_INK.ink,
				weight: 1.5,
				fillColor: "#fff",
				fillOpacity: 1,
			})
				.bindTooltip(`Spot ${i + 2}`)
				.addTo(layer);
		}
		L.circleMarker(guess, {
			radius: 7,
			color: "#fff",
			weight: 2.5,
			fillColor: MAP_INK.accent,
			fillOpacity: 1,
		})
			.bindTooltip("Where the photo was taken")
			.addTo(layer);
		const points = [
			guess,
			...ridge,
			...r.top.slice(0, 5).map((p) => [p.lat, p.lon] as [number, number]),
		];
		map.invalidateSize();
		map.fitBounds(L.latLngBounds(points), { padding: [30, 30], maxZoom: 14 });
	};

	const ready = loadLeaflet({ heat: true }).then((L) => {
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
	style:aspect-ratio={ratio}
	{@attach fromAction(locateMap, () => ({ result, heat }))}
	role="img"
	aria-label="Map: where the photo was taken, the direction it faces, the ridge it shows, and how well every spot matched"
></div>

<style>
	.map {
		width: 100%;
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
