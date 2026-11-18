<script lang="ts">
import LocateMap from "#lib/LocateMap.svelte";
import { svgPoints } from "#lib/locate.ts";
import ResultSummary from "#lib/ResultSummary.svelte";
import { resolve } from "$app/paths";
import type { PageProps } from "./$types";

let { data }: PageProps = $props();

let linkCopied = $state(false);
const taken = $derived(
	new Date(data.createdAt).toLocaleDateString(undefined, {
		year: "numeric",
		month: "short",
		day: "numeric",
	}),
);

const copyLink = async () => {
	await navigator.clipboard.writeText(window.location.href);
	linkCopied = true;
	setTimeout(() => {
		linkCopied = false;
	}, 1500);
};
</script>

<svelte:head>
	<title>Where this photo was taken</title>
	<!-- Unlisted: reachable by link only. -->
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<div class="page">
	<header>
		<a class="back" href={resolve("/locate")}>← Locate your own photo</a>
		<h1>Where this photo was taken</h1>
	</header>

	<div class="work">
		<section class="stage">
			<div class="frame">
				<img src={resolve("/p/[id]/photo", { id: data.id })} alt="A mountain skyline, with the traced ridge and the matching terrain horizon drawn on it" />
				<svg viewBox="0 0 {data.width} {data.height}" preserveAspectRatio="none">
					{#if data.box}
						{@const [x0, y0, x1, y1] = data.box}
						<path
							class="shade"
							fill-rule="evenodd"
							d="M0 0H{data.width}V{data.height}H0Z M{x0} {y0}V{y1}H{x1}V{y0}Z"
						/>
						<rect class="boxline" x={x0} y={y0} width={x1 - x0} height={y1 - y0} />
					{/if}
					<polyline class="auto" points={svgPoints(data.skyline)} />
					<polyline class="horizon" points={svgPoints(data.result.outline)} />
				</svg>
			</div>
			<div class="legend">
				<span class="key auto">— skyline the search used</span>
				{#if data.box}<span class="key box">▭ mountain box</span>{/if}
				<span class="key horizon">— terrain horizon from the best place</span>
			</div>
		</section>

		<aside class="side">
			<ResultSummary result={data.result} height={data.height} />
			<div class="share">
				<button type="button" class="go" onclick={copyLink}
					>{linkCopied ? "Link copied" : "Copy link to this result"}</button
				>
				<p class="dim">Located {taken}. Anyone with this link can see it.</p>
			</div>
		</aside>
	</div>

	<section class="mapbox">
		<LocateMap result={data.result} />
	</section>
</div>

<style>
	:global(body) {
		margin: 0;
		background: #0a0a0f;
		font-family: system-ui;
		color: #fff;
	}

	.page {
		display: flex;
		flex-direction: column;
		gap: 14px;
		box-sizing: border-box;
		max-width: 1280px;
		margin: 0 auto;
		padding: 12px 16px 40px;
	}

	header {
		display: flex;
		flex-wrap: wrap;
		gap: 12px;
		align-items: center;
	}

	h1 {
		flex: 1;
		margin: 0;
		font-size: 1.3rem;
	}

	a {
		color: #4facfe;
	}

	.back {
		font-size: 0.85rem;
	}

	.dim {
		margin: 0;
		color: #8a8a96;
		font-size: 0.8rem;
	}

	.work {
		display: grid;
		grid-template-columns: 1fr;
		gap: 16px;
		align-items: start;
	}

	@media (min-width: 960px) {
		.work {
			grid-template-columns: minmax(0, 1fr) 340px;
		}
	}

	.stage {
		display: flex;
		flex-direction: column;
		gap: 6px;
		align-items: center;
	}

	.frame {
		position: relative;
		max-width: 100%;
		line-height: 0;
	}

	.frame img {
		display: block;
		max-width: 100%;
		max-height: 68vh;
		border-radius: 10px;
		background: #15151d;
	}

	.frame svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}

	polyline {
		fill: none;
		vector-effect: non-scaling-stroke;
		stroke-linejoin: round;
		stroke-width: 3px;
	}

	.auto {
		stroke: #ffb020;
	}

	.horizon {
		stroke: #3ee0e0;
	}

	.shade {
		fill: #0a0a0f99;
	}

	.boxline {
		fill: none;
		stroke: #ff5cc8;
		stroke-width: 2px;
		stroke-dasharray: 8 5;
		vector-effect: non-scaling-stroke;
	}

	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: 14px;
		font-size: 0.8rem;
	}

	.key.auto {
		color: #ffb020;
	}

	.key.box {
		color: #ff5cc8;
	}

	.key.horizon {
		color: #3ee0e0;
	}

	.side {
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 14px;
		border: 1px solid #222;
		border-radius: 12px;
		background: #101018;
		font-size: 0.88rem;
	}

	.share {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding-top: 10px;
		border-top: 1px solid #222;
	}

	.go {
		width: 100%;
		padding: 10px 14px;
		border: 1px solid #3ecf6e;
		border-radius: 7px;
		background: #3ecf6e22;
		color: inherit;
		font: inherit;
		font-weight: 600;
		cursor: pointer;
	}

	.mapbox {
		position: relative;
	}
</style>
