<script lang="ts">
import type { Action } from "svelte/action";
import { fromAction } from "svelte/attachments";
import "leaflet/dist/leaflet.css";
import type { CircleMarker, Layer, Map as LeafletMap } from "leaflet";
import type { Coordinate, HeatSample } from "#lib/photo-match.ts";

let {
	samples,
	focus,
}: {
	samples: readonly HeatSample[];
	focus: Coordinate;
} = $props();

const heatMap: Action<HTMLDivElement, Coordinate> = (host, place) => {
	const heats = samples.map(
		(sample) =>
			[sample.latitude, sample.longitude, sample.weight] as [
				number,
				number,
				number,
			],
	);
	const corners = samples.map(
		(sample) => [sample.latitude, sample.longitude] as [number, number],
	);
	let map: LeafletMap | undefined;
	let marker: CircleMarker | undefined;
	let cancelled = false;

	const ready = import("leaflet").then(async (leaflet) => {
		const L = leaflet.default;
		(globalThis as typeof globalThis & { L: typeof L }).L = L;
		await import("leaflet.heat");
		if (cancelled || !host.isConnected || !place) return;

		map = L.map(host).setView([place.latitude, place.longitude], 12);
		L.tileLayer("https://tile.openstreetmap.org/{z}/{x}/{y}.png", {
			maxZoom: 19,
			attribution:
				'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>',
		}).addTo(map);

		const heatLayer = (
			L as typeof L & {
				heatLayer: (
					points: Array<[number, number, number]>,
					options: {
						radius: number;
						blur: number;
						minOpacity: number;
						max: number;
						gradient: Record<number, string>;
					},
				) => Layer;
			}
		).heatLayer;
		heatLayer(heats, {
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
		}).addTo(map);

		marker = L.circleMarker([place.latitude, place.longitude], {
			radius: 7,
			color: "#ffffff",
			weight: 2,
			fillColor: "#ffffff",
			fillOpacity: 0.9,
		}).addTo(map);

		map.fitBounds(L.latLngBounds(corners), { padding: [24, 24], maxZoom: 12 });
	});

	return {
		update(next) {
			if (!next) return;
			marker?.setLatLng([next.latitude, next.longitude]);
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
	{@attach fromAction(heatMap, () => focus)}
	role="img"
	aria-label="Heatmap of how well each ring viewpoint matches the photograph"
></div>

<style>
	.map {
		height: 420px;
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
