<script lang="ts">
import type { Attachment } from "svelte/attachments";
import "leaflet/dist/leaflet.css";
import outlines from "#lib/assets/counties.json";
import { addBasemap, loadLeaflet, MAP_INK } from "#lib/basemap.ts";

let { ratio = "16 / 9" }: { ratio?: string } = $props();

/** A GeoJSON FeatureCollection; JSON imports type `type` as a plain string, so it is cast once. */
const counties = outlines as unknown as Parameters<
	typeof import("leaflet").geoJSON
>[0];

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
		// Salt Lake, Utah and Davis counties: where a camera can stand (OpenStreetMap outlines).
		const shapes = L.geoJSON(counties, {
			style: {
				color: MAP_INK.accent,
				weight: 1.5,
				fillColor: MAP_INK.accent,
				fillOpacity: 0.12,
			},
			onEachFeature: (feature, layer) =>
				layer.bindTooltip(String(feature.properties?.name ?? "")),
		}).addTo(map);
		map.fitBounds(shapes.getBounds(), { padding: [12, 12] });
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
	aria-label="Map of the covered area: Salt Lake, Utah and Davis counties"
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
