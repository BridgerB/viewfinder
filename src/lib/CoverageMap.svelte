<script lang="ts">
import type { Attachment } from "svelte/attachments";
import "leaflet/dist/leaflet.css";
import { addBasemap, loadLeaflet, MAP_INK } from "#lib/basemap.ts";

let { ratio = "16 / 9" }: { ratio?: string } = $props();

/** The one terrain tile the locator has: 40–41° N, 111–112° W. */
const TILE: [[number, number], [number, number]] = [
	[40, -112],
	[41, -111],
];

const coverage: Attachment<HTMLDivElement> = (host) => {
	let cancelled = false;
	let remove = () => {};
	void loadLeaflet().then((L) => {
		if (cancelled || !host.isConnected) return;
		const map = L.map(host, {
			scrollWheelZoom: false,
			zoomControl: false,
			attributionControl: true,
		});
		addBasemap(L, map);
		L.rectangle(TILE, {
			color: MAP_INK.accent,
			weight: 1.5,
			fillColor: MAP_INK.accent,
			fillOpacity: 0.12,
		})
			.bindTooltip("Where Viewfinder works")
			.addTo(map);
		map.fitBounds(TILE, { padding: [12, 12] });
		remove = () => map.remove();
	});
	return () => {
		cancelled = true;
		remove();
	};
};
</script>

<div
	class="map"
	style:aspect-ratio={ratio}
	{@attach coverage}
	role="img"
	aria-label="Map of the covered area: the Wasatch Front, from Provo to Salt Lake City"
></div>

<style>
	.map {
		width: 100%;
		min-height: 200px;
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
