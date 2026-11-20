<script lang="ts">
import resultsData from "#lib/assets/results.json";
import { compass } from "#lib/places.ts";
import ResultsMap from "#lib/ResultsMap.svelte";
import { resolve } from "$app/paths";

type Example = {
	id: string;
	name: string;
	source: string;
	skyline: string;
	lens: string;
	note: string;
	truth: [number, number];
	guess: [number, number];
	errorKm: number;
	yaw: number;
	fov: number;
	pitch: number;
	rmse: number;
	loss: number;
	image: string;
	ridge: [number, number][];
	credit: {
		author: string;
		license: string;
		licenseUrl: string;
		page: string;
	} | null;
};

const examples = (resultsData as Example[]).toSorted(
	(a, b) => a.errorKm - b.errorKm,
);
let index = $state(0);
const current = $derived(examples[index] ?? examples[0]);

/** Green within 1 km of the real spot, amber within 10 km, red beyond. */
const level = (errorKm: number) =>
	errorKm <= 1 ? "good" : errorKm <= 10 ? "unsure" : "poor";
const step = (by: number) => {
	index = (index + by + examples.length) % examples.length;
};
const onkeydown = (event: KeyboardEvent) => {
	if (event.target instanceof HTMLInputElement) return;
	if (event.key === "ArrowLeft") step(-1);
	if (event.key === "ArrowRight") step(1);
};
</script>

<svelte:window {onkeydown} />

<svelte:head>
	<title>Examples · Viewfinder</title>
	<meta
		name="description"
		content="Photos from around Salt Lake City with known locations, and where Viewfinder placed each one."
	/>
</svelte:head>

<section class="intro">
	<div class="eyebrow">Examples</div>
	<h1>Photos from around Salt Lake.</h1>
	<p>
		Each photo was taken at a known spot. The map shows where Viewfinder placed it, how far that is from
		the real spot, and the ridge it matched.
	</p>
</section>

<div class="picker" role="tablist" aria-label="Example photos">
	{#each examples as ex, i (ex.id)}
		<button
			type="button"
			role="tab"
			aria-selected={i === index}
			class={["chip", level(ex.errorKm), { active: i === index }]}
			onclick={() => (index = i)}
		>
			<span>{ex.name}</span>
			<span class="miss">{ex.errorKm} km</span>
		</button>
	{/each}
</div>

<article class="example" style:--v="var(--{level(current.errorKm)})">
	<header>
		<div class="label">
			{current.errorKm <= 1 ? "Found" : current.errorKm <= 10 ? "Close" : "Missed"} · {current.errorKm} km
			from the real spot
		</div>
		<h2>{current.name}</h2>
	</header>

	<img
		class="rounded photo"
		src={current.image}
		alt="{current.name}, with the traced skyline in yellow and the terrain horizon in cyan"
	/>
	<div class="legend">
		<span><i class="line trace"></i>skyline</span>
		<span><i class="line horizon"></i>terrain</span>
	</div>

	<ResultsMap result={current} />
	<div class="legend">
		<span><i class="dot truth"></i>real spot</span>
		<span><i class="dot guess"></i>Viewfinder’s answer</span>
		<span><i class="line ridge"></i>the ridge in the photo</span>
	</div>

	<dl class="facts">
		<div><dt>Facing</dt><dd>{current.yaw}° {compass(current.yaw)}</dd></div>
		<div><dt>Lens</dt><dd>{current.fov}°</dd></div>
		<div><dt>Tilt</dt><dd>{current.pitch.toFixed(1)}°</dd></div>
		<div><dt>Fit</dt><dd>{current.loss.toFixed(3)}</dd></div>
	</dl>
	<p class="note">{current.note}</p>
	<p class="credit">
		{#if current.credit}
			Photo: <a href={current.credit.page} rel="external noopener">{current.credit.author}, Wikimedia Commons</a>,
			{#if current.credit.licenseUrl}
				<a href={current.credit.licenseUrl} rel="external noopener">{current.credit.license}</a>.
			{:else}
				{current.credit.license}.
			{/if}
			Resized, with skyline and horizon lines added.
		{:else}
			One of Viewfinder’s own test photos.
		{/if}
	</p>

	<div class="nav">
		<button type="button" class="textlink" onclick={() => step(-1)}>← Previous</button>
		<span class="count">{index + 1} of {examples.length}</span>
		<button type="button" class="textlink" onclick={() => step(1)}>Next →</button>
	</div>
</article>

<div class="footer">
	<span>Have a photo from the Wasatch Front?</span>
	<a class="pill big" href={resolve("/")}>Try your own</a>
</div>

<style>
	.intro {
		display: grid;
		gap: 10px;
		padding: 48px 0 24px;
	}

	h1 {
		margin: 0;
		font: 400 clamp(40px, 7vw, 68px) / 1.02 var(--serif);
		letter-spacing: -0.025em;
	}

	.intro p {
		margin: 6px 0 0;
		max-width: 56ch;
		color: var(--ink-2);
		font-size: 18px;
	}

	.picker {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	.chip {
		display: inline-flex;
		gap: 8px;
		padding: 7px 14px;
		border: 1px solid var(--rule-2);
		border-radius: 99px;
		background: var(--paper);
		color: var(--ink);
		font-size: 14px;
	}

	.chip:hover {
		border-color: var(--ink);
	}

	.chip.active {
		border-color: var(--ink);
		background: var(--ink);
		color: #fff;
	}

	.chip .miss {
		font-variant-numeric: tabular-nums;
	}

	.chip.good .miss {
		color: var(--good);
	}

	.chip.unsure .miss {
		color: var(--unsure);
	}

	.chip.poor .miss {
		color: var(--poor);
	}

	.chip.active .miss {
		color: #fff;
		opacity: 0.75;
	}

	.example {
		display: grid;
		gap: 14px;
		margin-top: 32px;
	}

	.example header {
		display: grid;
		gap: 8px;
		padding-bottom: 10px;
	}

	.label {
		color: var(--v);
		font: 700 13px var(--sans);
		letter-spacing: 0.12em;
		text-transform: uppercase;
	}

	h2 {
		margin: 0;
		font: 400 clamp(32px, 5vw, 46px) / 1.05 var(--serif);
		letter-spacing: -0.02em;
	}

	.photo {
		display: block;
		width: 100%;
		height: auto;
	}

	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 20px;
		margin-bottom: 14px;
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

	.line.ridge {
		background: var(--ink);
	}

	.dot {
		width: 11px;
		height: 11px;
		border-radius: 50%;
		border: 2px solid #fff;
		box-shadow: 0 0 0 1px var(--ink);
	}

	.dot.truth {
		background: var(--ink);
	}

	.dot.guess {
		background: var(--accent);
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

	.note {
		margin: 6px 0 0;
		max-width: 62ch;
		color: var(--ink-2);
	}

	.credit {
		margin: 0;
		color: var(--faint);
		font-size: 13px;
		overflow-wrap: anywhere;
	}

	.credit a {
		color: var(--muted);
		border-bottom-color: var(--rule-2);
	}

	.nav {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 12px;
		margin-top: 12px;
	}

	.count {
		color: var(--faint);
		font-size: 14px;
		font-variant-numeric: tabular-nums;
	}

	.footer {
		display: flex;
		flex-wrap: wrap;
		gap: 16px 24px;
		align-items: center;
		justify-content: space-between;
		margin-top: 44px;
		padding-top: 20px;
		border-top: 1px solid var(--rule);
		color: var(--muted);
		font-size: 14px;
	}
</style>
