<script lang="ts">
import type { LayerGroup, Map as LeafletMap } from "leaflet";
import type { Action } from "svelte/action";
import { fromAction } from "svelte/attachments";
import "leaflet/dist/leaflet.css";
import type { Located } from "#lib/locate.ts";
import { searchHeat } from "#lib/photo-match.ts";

let { result }: { result: Located } = $props();

type Leaflet = typeof import("leaflet");
type HeatLayer = (
	points: Array<[number, number, number]>,
	options: Record<string, unknown>,
) => { addTo: (layer: LayerGroup) => unknown };

const locateMap: Action<HTMLDivElement, Located> = (host, first) => {
	let map: LeafletMap | undefined;
	let layer: LayerGroup | undefined;
	let L: Leaflet | undefined;
	let current = first;
	let cancelled = false;

	const draw = (r: Located) => {
		current = r;
		if (!map || !layer || !L) return;
		layer.clearLayers();
		const guess: [number, number] = [r.best.lat, r.best.lon];
		const heat = searchHeat(r.surface);
		(L as Leaflet & { heatLayer: HeatLayer })
			.heatLayer(
				heat.samples.map((s) => [s.latitude, s.longitude, s.weight]),
				{
					radius: 22,
					blur: 16,
					minOpacity: 0.35,
					max: 1,
					gradient: {
						0.2: "#1d4e89",
						0.5: "#3ee0e0",
						0.75: "#ffb020",
						1: "#fff6d8",
					},
				},
			)
			.addTo(layer);
		const ridge = r.ridge.map(([lat, lon]) => [lat, lon] as [number, number]);
		if (ridge.length > 1) {
			L.polygon([guess, ...ridge, guess], {
				stroke: false,
				fillColor: "#4facfe",
				fillOpacity: 0.18,
			}).addTo(layer);
			L.polyline(ridge, { color: "#4facfe", weight: 3 })
				.bindTooltip("The ridge the skyline sits on")
				.addTo(layer);
		}
		for (const [i, p] of r.top.slice(1, 5).entries()) {
			L.circleMarker([p.lat, p.lon], {
				radius: 5,
				color: "#111111",
				weight: 1,
				fillColor: "#ffb020",
				fillOpacity: 0.9,
			})
				.bindTooltip(`Candidate ${i + 2}, loss ${p.loss.toFixed(2)}`)
				.addTo(layer);
		}
		L.circleMarker(guess, {
			radius: 8,
			color: "#111111",
			weight: 2,
			fillColor: "#3ecf6e",
			fillOpacity: 1,
		})
			.bindTooltip(`Best guess, loss ${r.best.loss.toFixed(2)}`)
			.addTo(layer);
		const points = [
			guess,
			...ridge,
			...r.top.slice(0, 5).map((p) => [p.lat, p.lon] as [number, number]),
		];
		map.invalidateSize();
		map.fitBounds(L.latLngBounds(points), { padding: [30, 30], maxZoom: 14 });
	};

	const ready = import("leaflet").then(async (leaflet) => {
		L = leaflet.default;
		(globalThis as typeof globalThis & { L: Leaflet }).L = L;
		await import("leaflet.heat");
		if (cancelled || !host.isConnected) return;
		map = L.map(host);
		L.tileLayer("https://tile.openstreetmap.org/{z}/{x}/{y}.png", {
			maxZoom: 19,
			attribution:
				'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>',
		}).addTo(map);
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
	{@attach fromAction(locateMap, () => result)}
	role="img"
	aria-label="Map of the search: best guess, other candidates, heatmap of good fits, and the ridge the photo shows"
></div>

<style>
	.map {
		height: 520px;
		border-radius: 8px;
		overflow: hidden;
		background: #111;
	}

	.map :global(.leaflet-control-attribution) {
		background: rgba(10, 10, 15, 0.72);
		color: #888;
		font-size: 10px;
	}

	.map :global(.leaflet-control-attribution a) {
		color: #4facfe;
	}
</style>
