<script lang="ts">
	import type { Action } from 'svelte/action';
	import { fromAction } from 'svelte/attachments';
	import 'leaflet/dist/leaflet.css';
	import type { LayerGroup, Map as LeafletMap } from 'leaflet';

	type Result = {
		id: string;
		name: string;
		truth: [number, number];
		guess: [number, number];
		errorKm: number;
		yaw: number;
		fov: number;
		ridge: [number, number][];
	};

	let { result }: { result: Result } = $props();

	const colorOf = (errorKm: number) =>
		errorKm <= 1 ? '#3ecf6e' : errorKm <= 10 ? '#ffb020' : '#ff4d4d';

	const resultsMap: Action<HTMLDivElement, Result> = (host, first) => {
		let map: LeafletMap | undefined;
		let layer: LayerGroup | undefined;
		let current = first;
		let cancelled = false;

		const draw = (r: Result) => {
			current = r;
			if (!map || !layer) return;
			const L = (globalThis as typeof globalThis & { L?: typeof import('leaflet') }).L;
			if (!L) return;
			layer.clearLayers();
			const color = colorOf(r.errorKm);
			L.polygon([r.guess, ...r.ridge, r.guess], {
				stroke: false,
				fillColor: color,
				fillOpacity: 0.18
			}).addTo(layer);
			L.polyline(r.ridge, { color, weight: 3 })
				.bindTooltip('The ridge its skyline sits on')
				.addTo(layer);
			L.polyline([r.truth, r.guess], { color, weight: 2, dashArray: '6 6' }).addTo(layer);
			L.circleMarker(r.truth, {
				radius: 7,
				color: '#ffffff',
				weight: 2,
				fillColor: '#2b6cff',
				fillOpacity: 1
			})
				.bindTooltip('Where the photo was taken')
				.addTo(layer);
			L.circleMarker(r.guess, {
				radius: 7,
				color: '#111111',
				weight: 2,
				fillColor: color,
				fillOpacity: 1
			})
				.bindTooltip(`Search guess, ${r.errorKm} km off`)
				.addTo(layer);
			const points = [r.truth, ...r.ridge];
			const dLat = Math.max(...points.map((p) => Math.abs(p[0] - r.guess[0])));
			const dLon = Math.max(...points.map((p) => Math.abs(p[1] - r.guess[1])));
			map.invalidateSize();
			map.fitBounds(
				[
					[r.guess[0] - dLat, r.guess[1] - dLon],
					[r.guess[0] + dLat, r.guess[1] + dLon]
				],
				{ padding: [30, 30], maxZoom: 14 }
			);
		};

		const ready = import('leaflet').then((leaflet) => {
			const L = leaflet.default;
			(globalThis as typeof globalThis & { L: typeof L }).L = L;
			if (cancelled || !host.isConnected) return;
			map = L.map(host);
			L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
				maxZoom: 19,
				attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
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
			}
		};
	};
</script>

<div
	class="map"
	{@attach fromAction(resultsMap, () => result)}
	role="img"
	aria-label="Map of {result.name}: true camera spot, search guess, and the ridge it sees"
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
