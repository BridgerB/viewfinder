<script lang="ts">
import type { Snippet } from "svelte";
import LocateMap from "#lib/LocateMap.svelte";
import {
	kmBetween,
	type Located,
	shapeOf,
	spreadKm,
	verdict,
} from "#lib/locate.ts";
import { compass, latLonText, placeName } from "#lib/places.ts";

let {
	result,
	height,
	photo,
	editable = false,
	busy = false,
	shareUrl = "",
	shareNote = "",
	tools,
}: {
	result: Located;
	/** The photo's height in pixels, to judge how much shape the skyline has. */
	height: number;
	/** The photo with its overlays. */
	photo: Snippet;
	/** Whether the box can be dragged here (the person's own upload, not a shared link). */
	editable?: boolean;
	/** A new search is running: the numbers below are about to change. */
	busy?: boolean;
	shareUrl?: string;
	/** Shown in place of the link when it could not be made, or while it is being saved. */
	shareNote?: string;
	/** Extra controls under the camera facts, such as the lens. */
	tools?: Snippet;
} = $props();

const spread = $derived(spreadKm(result.top));
const judged = $derived(
	verdict(result.best, spread, shapeOf(result.skyline, height)),
);
const level = $derived(judged.level);
const color = $derived(`var(--${level})`);
const heat = $derived(
	level === "good" ? "#2f7d4a" : level === "unsure" ? "#9a6200" : "#b3261e",
);
const label = $derived(
	level === "good"
		? "Good match"
		: level === "unsure"
			? "Not sure"
			: "Poor match",
);
const at = $derived([result.best.lat, result.best.lon] as const);
const place = $derived(level === "poor" ? "Unclear" : placeName(at));
const hint = $derived(
	level === "unsure"
		? "Tighten the box around the ridge."
		: "Drag the box to cut out trees, roofs or clouds.",
);
const coords = $derived(
	`${result.best.lat.toFixed(5)}, ${result.best.lon.toFixed(5)}`,
);
const distances = $derived(
	result.top.slice(0, 5).map((p) => kmBetween(at, [p.lat, p.lon])),
);
const candNote = $derived(
	spread <= 1
		? "All five agree."
		: spread <= 3
			? `All five within ${spread.toFixed(1)} km.`
			: level === "poor"
				? `Spread over ${spread.toFixed(0)} km. Don’t trust this pin.`
				: `Spread over ${spread.toFixed(0)} km. Treat it as a region.`,
);

let copied = $state<"" | "coords" | "link">("");
let copyTimer: ReturnType<typeof setTimeout> | undefined;
const copy = async (what: "coords" | "link") => {
	try {
		await navigator.clipboard.writeText(what === "coords" ? coords : shareUrl);
		copied = what;
		clearTimeout(copyTimer);
		copyTimer = setTimeout(() => {
			copied = "";
		}, 1600);
	} catch {
		// The clipboard refused; the text is still on the page to select.
	}
};
</script>

<div class="result" style:--v={color}>
	<div class={["head", { busy }]}>
		<div class="label">
			<span class={["mark", level]} aria-hidden="true"
				>{level === "good" ? "✓" : level === "unsure" ? "!" : "✕"}</span
			>{label}
		</div>
		<h1>{place}</h1>
		{#if level !== "good"}
			<p class="why">{judged.text}</p>
			{#if editable && !busy}<p class="hint">{hint}</p>{/if}
		{/if}
	</div>

	{@render photo()}

	<div class="legend">
		<span><i class="line trace"></i>your skyline</span>
		<span><i class="line horizon"></i>terrain</span>
		{#if editable}<span><i class="square"></i>drag to adjust</span>{/if}
	</div>

	<div class={["fades", { busy }]}>
		<section class="where">
			<div class="coords">{latLonText(at)}</div>
			<div class="actions">
				<button type="button" class="pill" onclick={() => copy("coords")}
					>{copied === "coords" ? "Copied ✓" : "Copy coordinates"}</button
				>
				<a href="https://www.google.com/maps?q={coords.replace(' ', '')}" target="_blank" rel="noopener external"
					>Open in Google Maps</a
				>
			</div>
		</section>

		<LocateMap {result} {heat} />

		<dl class="facts">
			<div><dt>Facing</dt><dd>{result.best.yaw}° {compass(result.best.yaw)}</dd></div>
			<div><dt>Lens</dt><dd>{result.best.fov}°</dd></div>
			<div><dt>Tilt</dt><dd>{result.best.pitch.toFixed(1)}°</dd></div>
			<div><dt>Ground</dt><dd>{Math.round(result.best.elev).toLocaleString("en-US")} m</dd></div>
			<div><dt>Fit</dt><dd>{result.best.loss.toFixed(3)}</dd></div>
			<div><dt>Search</dt><dd>{Math.max(1, Math.round(result.seconds))} s</dd></div>
		</dl>
		{@render tools?.()}

		<section class="cands">
			<div class="small">Top 5 spots · distance from answer</div>
			<div class="chips">
				{#each distances as km, i (i)}
					<span class="chip">{km.toFixed(2)} km</span>
				{/each}
			</div>
			<div class="note">{candNote}</div>
		</section>
	</div>

	{#if shareUrl || shareNote}
		<section class="share">
			<div class="eyebrow">Share</div>
			{#if shareUrl}
				<div class="shareline">
					<span class="url">{shareUrl.replace(/^https?:\/\//, "")}</span>
					<button type="button" class="pill dark" onclick={() => copy("link")}
						>{copied === "link" ? "Copied ✓" : "Copy link"}</button
					>
				</div>
				<div class="note">Anyone with the link can see the photo.</div>
			{:else}
				<div class="note">{shareNote}</div>
			{/if}
		</section>
	{/if}
</div>

<style>
	.result {
		display: grid;
	}

	.head {
		display: grid;
		gap: 10px;
		padding: 36px 0 24px;
		transition: opacity 0.2s;
	}

	.busy {
		opacity: 0.45;
	}

	.label {
		display: flex;
		gap: 10px;
		align-items: center;
		color: var(--v);
		font: 700 13px var(--sans);
		letter-spacing: 0.12em;
		text-transform: uppercase;
	}

	.mark {
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		background: var(--v);
		color: #fff;
		font-size: 12px;
		letter-spacing: 0;
	}

	.mark.good {
		border-radius: 50%;
	}

	.mark.unsure {
		width: 24px;
		clip-path: polygon(50% 0, 100% 100%, 0 100%);
		place-items: end center;
	}

	h1 {
		margin: 0;
		font: 400 clamp(36px, 6vw, 56px) / 1.05 var(--serif);
		letter-spacing: -0.02em;
	}

	.why {
		margin: 0;
		max-width: 60ch;
		color: var(--ink-2);
	}

	.hint {
		margin: 0;
		color: var(--v);
		font-weight: 700;
	}

	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 20px;
		padding-top: 12px;
		color: var(--muted);
		font-size: 13px;
	}

	.legend span {
		display: inline-flex;
		align-items: center;
		gap: 7px;
	}

	.line {
		width: 16px;
		height: 3px;
	}

	.line.trace {
		background: var(--trace);
	}

	.line.horizon {
		background: var(--horizon);
	}

	.square {
		width: 11px;
		height: 11px;
		border: 2px solid var(--box);
		border-radius: 2px;
	}

	.fades {
		display: grid;
		gap: 28px;
		margin-top: 40px;
		transition: opacity 0.2s;
	}

	.where {
		display: grid;
		gap: 14px;
		padding-top: 24px;
		border-top: 1px solid var(--rule);
	}

	.coords {
		font: 400 clamp(26px, 5vw, 34px) / 1.1 var(--serif);
		font-variant-numeric: tabular-nums;
		overflow-wrap: anywhere;
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 10px 22px;
		align-items: center;
	}

	.actions a {
		font-weight: 500;
	}

	.facts {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
		margin: 0;
		border-top: 1px solid var(--rule);
	}

	.facts div {
		padding: 14px 0;
		border-bottom: 1px solid var(--rule);
	}

	.facts dt {
		color: var(--faint);
		font-size: 12px;
	}

	.facts dd {
		margin: 0;
		font: 400 22px var(--serif);
		font-variant-numeric: tabular-nums;
	}

	.cands {
		display: grid;
		gap: 8px;
	}

	.small {
		color: var(--faint);
		font-size: 12px;
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	.chip {
		padding: 6px 14px;
		border: 1px solid var(--rule-2);
		border-radius: 99px;
		font-size: 14px;
		font-variant-numeric: tabular-nums;
	}

	.note {
		color: var(--muted);
		font-size: 14px;
	}

	.share {
		display: grid;
		gap: 10px;
		margin-top: 36px;
		padding: 20px;
		border-radius: 16px;
		background: var(--soft);
	}

	.shareline {
		display: flex;
		flex-wrap: wrap;
		gap: 10px 16px;
		align-items: center;
		justify-content: space-between;
	}

	.url {
		min-width: 0;
		font: 500 13px var(--mono);
		overflow-wrap: anywhere;
	}

	.share .note {
		font-size: 13px;
	}
</style>
