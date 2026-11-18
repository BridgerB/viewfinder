<script lang="ts">
import type { Snippet } from "svelte";
import {
	kmBetween,
	type Located,
	shapeOf,
	spreadKm,
	verdict,
} from "#lib/locate.ts";

let {
	result,
	height,
	afterVerdict,
}: {
	result: Located;
	/** The photo's height in pixels, to judge how much shape the skyline has. */
	height: number;
	/** Shown right under the verdict, e.g. a prompt to fix the skyline when the match is weak. */
	afterVerdict?: Snippet<[level: "good" | "unsure" | "poor"]>;
} = $props();

const judged = $derived(
	verdict(result.best, spreadKm(result.top), shapeOf(result.skyline, height)),
);
const coords = $derived(
	`${result.best.lat.toFixed(5)}, ${result.best.lon.toFixed(5)}`,
);
let copied = $state(false);

const copy = async () => {
	await navigator.clipboard.writeText(coords);
	copied = true;
	setTimeout(() => {
		copied = false;
	}, 1500);
};
</script>

<div class={["verdict", judged.level]}>{judged.text}</div>
{@render afterVerdict?.(judged.level)}
<div class="where">
	<span class="coords">{coords}</span>
	<div class="row">
		<button type="button" class="ghost" onclick={copy}>{copied ? "Copied" : "Copy"}</button>
		<a
			class="button ghost"
			href="https://www.google.com/maps?q={coords.replace(' ', '')}"
			rel="external noopener"
			target="_blank">Google Maps</a
		>
	</div>
</div>
<dl class="facts">
	<dt>Facing</dt>
	<dd>{result.best.yaw}°</dd>
	<dt>Lens</dt>
	<dd>{result.best.fov}°</dd>
	<dt>Tilt</dt>
	<dd>{result.best.pitch.toFixed(1)}°</dd>
	<dt>Ground</dt>
	<dd>{result.best.elev.toFixed(0)} m</dd>
	<dt>Fit</dt>
	<dd>loss {result.best.loss.toFixed(3)}</dd>
	<dt>Time</dt>
	<dd>{result.seconds.toFixed(1)} s</dd>
</dl>
<details class="more">
	<summary>Best candidates</summary>
	<ol class="top">
		{#each result.top.slice(0, 5) as p, i (i)}
			<li>
				<span>loss {p.loss.toFixed(3)}, facing {p.yaw}°</span>
				<span class="dim"
					>{i === 0
						? "best"
						: `${kmBetween([result.best.lat, result.best.lon], [p.lat, p.lon]).toFixed(2)} km away`}</span
				>
			</li>
		{/each}
	</ol>
</details>

<style>
	.verdict {
		padding: 10px 12px;
		border-radius: 8px;
		line-height: 1.4;
	}

	.verdict.good {
		border: 1px solid #3ecf6e;
		background: #3ecf6e1a;
	}

	.verdict.unsure {
		border: 1px solid #ffb020;
		background: #ffb0201a;
	}

	.verdict.poor {
		border: 1px solid #ff6b6b;
		background: #ff6b6b1a;
	}

	.where {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.coords {
		font-size: 1.15rem;
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}

	.row {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	button,
	.button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 7px 12px;
		border: 1px solid #333;
		border-radius: 7px;
		background: #15151d;
		color: inherit;
		font: inherit;
		font-size: 0.85rem;
		text-decoration: none;
		cursor: pointer;
	}

	button:hover,
	.button:hover {
		border-color: #555;
	}

	.facts {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 4px 14px;
		margin: 0;
	}

	.facts dt,
	.dim,
	.more summary {
		color: #8a8a96;
	}

	.facts dd {
		margin: 0;
		font-variant-numeric: tabular-nums;
	}

	.more summary {
		cursor: pointer;
	}

	.more[open] summary {
		margin-bottom: 8px;
	}

	.top {
		margin: 0;
		padding-left: 20px;
		line-height: 1.6;
	}

	.top li span + span {
		margin-left: 6px;
	}
</style>
