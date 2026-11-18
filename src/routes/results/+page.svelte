<script lang="ts">
	import { resolve } from '$app/paths';
	import resultsData from '$lib/assets/results.json';
	import ResultsMap from '$lib/ResultsMap.svelte';

	type Result = {
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
		credit: { author: string; license: string; licenseUrl: string; page: string } | null;
	};

	const results = (resultsData as Result[]).toSorted((a, b) => a.errorKm - b.errorKm);
	let index = $state(0);
	const current = $derived(results[index] ?? results[0]);

	const verdict = (errorKm: number) =>
		errorKm <= 1 ? 'found' : errorKm <= 10 ? 'drifted' : 'failed';
	const step = (by: number) => {
		index = (index + by + results.length) % results.length;
	};
	const onkeydown = (event: KeyboardEvent) => {
		if (event.key === 'ArrowLeft') step(-1);
		if (event.key === 'ArrowRight') step(1);
	};
</script>

<svelte:window {onkeydown} />

<svelte:head>
	<title>Photo locator results</title>
</svelte:head>

<div class="container">
	<p class="back"><a href={resolve('/')}>Home</a></p>
	<h1>Where the search puts each photo</h1>
	<p class="subtitle">
		Blue dot: where the photo was really taken. Colored dot: the search's guess. The shaded wedge runs
		from the guess out to the ridge its skyline sits on (solid line), one point per degree of view.
		Dashed line: the miss. Green within 1 km, amber within 10 km, red beyond. Pick a photo below, or
		use the arrow keys.
	</p>

	<div class="picker">
		<button type="button" class="nav" onclick={() => step(-1)} aria-label="Previous photo">‹</button>
		{#each results as r, i (r.id)}
			<button
				type="button"
				class={['pick', verdict(r.errorKm), { active: i === index }]}
				onclick={() => (index = i)}
			>
				<span>{r.name}</span>
				<span class="miss">{r.errorKm} km</span>
			</button>
		{/each}
		<button type="button" class="nav" onclick={() => step(1)} aria-label="Next photo">›</button>
	</div>

	<div class={['view', verdict(current.errorKm)]}>
		<ResultsMap result={current} />
		<div class="card">
			<img src={current.image} alt="{current.name} with traced skyline and fitted horizon" />
			<div class="text">
				<div class="title">
					<span>{current.name}</span>
					<span class="miss">{current.errorKm} km off</span>
				</div>
				<div class="facts">
					facing {current.yaw}° · lens {current.fov}° · tilt {current.pitch.toFixed(1)}° · fit
					{current.rmse.toFixed(2)}° · loss {current.loss.toFixed(2)}
				</div>
				<div class="facts">skyline {current.skyline} · real lens {current.lens}</div>
				<div class="note">{current.note}</div>
				<div class="facts">
					Orange: skyline traced from the photo. Cyan: terrain horizon from the guessed spot.
				</div>
				{#if current.credit}
					<div class="source">
						Photo: <a href={current.credit.page} rel="external noopener">{current.credit.author}, Wikimedia
							Commons</a
						>,
						{#if current.credit.licenseUrl}
							<a href={current.credit.licenseUrl} rel="external noopener">{current.credit.license}</a>.
						{:else}
							{current.credit.license}.
						{/if}
						Resized, with skyline and horizon lines added.
					</div>
				{:else}
					<div class="source">{current.source}</div>
				{/if}
			</div>
		</div>
	</div>
</div>

<style>
	:global(body) {
		margin: 0;
		background: #0a0a0f;
		font-family: system-ui;
		color: #fff;
	}

	.container {
		display: flex;
		flex-direction: column;
		gap: 10px;
		box-sizing: border-box;
		max-width: 1200px;
		margin: 0 auto;
		padding: 12px;
	}

	.back {
		margin: 0;
	}

	a {
		color: #4facfe;
	}

	h1 {
		margin: 0;
		font-size: 1.3rem;
	}

	.subtitle {
		margin: 0;
		color: #aaa;
		font-size: 0.85rem;
		line-height: 1.4;
	}

	.picker {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.pick,
	.nav {
		display: flex;
		gap: 8px;
		padding: 6px 10px;
		border: 2px solid #222;
		border-radius: 6px;
		background: #111;
		color: inherit;
		font: inherit;
		font-size: 0.8rem;
		cursor: pointer;
	}

	.nav {
		font-size: 1rem;
		font-weight: bold;
	}

	.pick.active {
		border-color: #fff;
	}

	.view {
		display: grid;
		grid-template-columns: 1fr;
		gap: 12px;
	}

	@media (min-width: 900px) {
		.view {
			grid-template-columns: 3fr 2fr;
		}
	}

	.card {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border: 2px solid #222;
		border-radius: 8px;
		background: #111;
	}

	.found .card {
		border-color: #3ecf6e55;
	}

	.drifted .card {
		border-color: #ffb02055;
	}

	.failed .card {
		border-color: #ff4d4d55;
	}

	.card img {
		display: block;
		width: 100%;
		height: auto;
	}

	.text {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 10px;
	}

	.title {
		display: flex;
		justify-content: space-between;
		gap: 8px;
		font-weight: 600;
	}

	.found .miss {
		color: #3ecf6e;
	}

	.drifted .miss {
		color: #ffb020;
	}

	.failed .miss {
		color: #ff4d4d;
	}

	.facts {
		color: #aaa;
		font-size: 0.8rem;
	}

	.note {
		color: #ddd;
		font-size: 0.82rem;
	}

	.source {
		color: #666;
		font-size: 0.72rem;
		overflow-wrap: anywhere;
	}

	@media (min-width: 640px) {
		.container {
			padding: 20px;
		}
	}
</style>
