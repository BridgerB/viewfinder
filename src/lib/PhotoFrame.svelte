<script lang="ts">
import type { Snippet } from "svelte";
import { type Box, type Point, svgPoints } from "#lib/locate.ts";

let {
	src,
	width,
	height,
	alt,
	skyline = [],
	horizon = [],
	box = null,
	boxMode = "none",
	hidden = false,
	onerror,
	onload,
	children,
}: {
	src: string;
	/** The photo's size in pixels: the overlays are drawn in these coordinates. */
	width: number;
	height: number;
	alt: string;
	/** The skyline traced from the photo (yellow). */
	skyline?: readonly Point[];
	/** The terrain horizon computed at the answer (cyan). */
	horizon?: readonly Point[];
	box?: Box | null;
	/** "edit" draws the box solid with corner handles, "view" dashed, "none" not at all. */
	boxMode?: "edit" | "view" | "none";
	/** Hide the image itself (a HEIC preview the browser cannot show) but keep the frame. */
	hidden?: boolean;
	onerror?: () => void;
	onload?: () => void;
	/** Anything laid over the photo: a drag surface, a status pill. */
	children?: Snippet;
} = $props();

/** Corner handle half-size in image pixels: about 5 screen pixels on a typical frame. */
const r = $derived(width / 150);
</script>

<div class="stage">
	<div class="frame">
		<img {src} {alt} class={{ hidden }} {onerror} {onload} draggable="false" />
		<svg viewBox="0 0 {width} {height}" preserveAspectRatio="none" aria-hidden="true">
			{#if horizon.length > 1}
				<polyline class="shadow wide" points={svgPoints(horizon)} />
				<polyline class="horizon" points={svgPoints(horizon)} />
			{/if}
			{#if skyline.length > 1}
				<polyline class="shadow" points={svgPoints(skyline)} />
				<polyline class="trace" points={svgPoints(skyline)} />
			{/if}
			{#if box && boxMode !== "none"}
				{@const [x0, y0, x1, y1] = box}
				<rect class="boxshadow" x={x0} y={y0} width={x1 - x0} height={y1 - y0} />
				<rect class={["boxline", boxMode]} x={x0} y={y0} width={x1 - x0} height={y1 - y0} />
				{#if boxMode === "edit"}
					{#each [[x0, y0], [x1, y0], [x0, y1], [x1, y1]] as [hx, hy], i (i)}
						<rect class="handle" x={hx - r} y={hy - r} width={r * 2} height={r * 2} rx={r / 4} />
					{/each}
				{/if}
			{/if}
		</svg>
		{@render children?.()}
	</div>
</div>

<style>
	.stage {
		display: flex;
		justify-content: center;
		border-radius: 14px;
		background: var(--soft);
	}

	.frame {
		position: relative;
		max-width: 100%;
		line-height: 0;
	}

	img {
		display: block;
		max-width: 100%;
		max-height: 72vh;
		min-height: 160px;
		border-radius: 14px;
		user-select: none;
		-webkit-user-select: none;
	}

	img.hidden {
		visibility: hidden;
	}

	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
		pointer-events: none;
	}

	polyline,
	rect {
		fill: none;
		vector-effect: non-scaling-stroke;
		stroke-linejoin: round;
	}

	.shadow {
		stroke: #000;
		stroke-opacity: 0.45;
		stroke-width: 5px;
	}

	.shadow.wide {
		stroke-width: 6.5px;
	}

	.horizon {
		stroke: var(--horizon);
		stroke-width: 3.5px;
	}

	.trace {
		stroke: var(--trace);
		stroke-width: 2.2px;
	}

	.boxshadow {
		stroke: #000;
		stroke-opacity: 0.4;
		stroke-width: 4px;
	}

	.boxline {
		stroke: var(--box);
		stroke-width: 2px;
	}

	.boxline.view {
		stroke-dasharray: 5 4;
	}

	.handle {
		fill: var(--box);
		stroke: #fff;
		stroke-width: 1.2px;
	}
</style>
