<script lang="ts">
	import { resolve } from '$app/paths';
	import searchData from '$lib/assets/sandy-search.json';
	import skylineData from '$lib/assets/sandy-skyline.json';
	import matchData from '$lib/assets/sandy-match.json';
	import * as photoMatch from '$lib/photo-match';
	import MatchMap from '$lib/MatchMap.svelte';

	const search = photoMatch.searchHeat(searchData as photoMatch.SearchHit[]);
	const frame = skylineData as {
		width: number;
		height: number;
		points: readonly (readonly [number, number])[];
	};
	const skyline = frame.points.map(([x, y]) => `${x},${y}`).join(' ');
	const match = matchData as {
		yaw: number;
		fov: number;
		roll: number;
		pitch: number;
		rmse: number;
		elev: number;
		horizon: readonly (readonly [number, number])[];
	};
	const horizon = match.horizon.map(([x, y]) => `${x},${y}`).join(' ');
</script>

<svelte:head>
	<title>Sandy photo match</title>
</svelte:head>

<div class="container">
	<p class="back"><a href={resolve('/')}>Home</a></p>
	<h1>Photo match</h1>

	<div class="frame">
		<img src="/sandy.jpg" alt="Photograph of a Wasatch ridge above a valley" />
		<svg viewBox="0 0 {frame.width} {frame.height}" preserveAspectRatio="xMidYMid meet">
			<polyline class="skyline" points={skyline} />
			<polyline class="model" points={horizon} />
		</svg>
	</div>

	<ul class="legend">
		<li><span class="swatch skyline"></span> Skyline traced from the photo</li>
		<li><span class="swatch model"></span> Terrain horizon from the closest match</li>
	</ul>
	<p class="subtitle">
		Camera facing {match.yaw.toFixed(0)}°, {match.fov.toFixed(0)}° field of view, tilted
		{match.pitch.toFixed(1)}° up, standing at {match.elev.toFixed(0)} m. The two lines differ by
		{match.rmse.toFixed(2)}° on average.
	</p>

	<h2>Likely places</h2>
	<p class="subtitle">
		Hotter ground matches this photograph more closely. The dot is the closest match,
		{photoMatch.coordinateText(search.best)}.
	</p>
	<MatchMap samples={search.samples} focus={search.best} />
	<ul class="legend">
		<li><span class="swatch heat"></span> Stronger match</li>
		<li><span class="swatch here"></span> Closest match</li>
	</ul>
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
		gap: 8px;
		box-sizing: border-box;
		max-width: 960px;
		margin: 0 auto;
		padding: 12px;
	}

	.back {
		margin: 0;
	}

	a {
		color: #4facfe;
	}

	a:hover {
		color: #00f2fe;
	}

	h1,
	h2 {
		margin: 0;
		text-align: center;
	}

	h1 {
		font-size: 1.2rem;
	}

	h2 {
		margin-top: 8px;
		font-size: 0.95rem;
		font-weight: 600;
	}

	.subtitle {
		margin: 0;
		text-align: center;
		color: #aaa;
		font-size: 0.85rem;
	}

	.frame {
		position: relative;
		margin-top: 8px;
	}

	.frame img,
	.frame svg {
		display: block;
		width: 100%;
		height: auto;
	}

	.frame svg {
		position: absolute;
		inset: 0;
		pointer-events: none;
	}

	polyline {
		fill: none;
		stroke-linejoin: round;
		stroke-linecap: round;
	}

	.skyline {
		stroke: #ffb020;
		stroke-width: 2.5;
		vector-effect: non-scaling-stroke;
	}

	.legend {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: 16px;
		margin: 0;
		padding: 0;
		list-style: none;
		color: #aaa;
		font-size: 0.8rem;
	}

	.legend li {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.swatch {
		width: 18px;
		height: 3px;
		border-radius: 2px;
	}

	.swatch.skyline {
		background: #ffb020;
	}

	.model {
		stroke: #3ee0e0;
		stroke-width: 2.5;
		vector-effect: non-scaling-stroke;
	}

	.swatch.model {
		background: #3ee0e0;
	}

	.swatch.heat {
		background: linear-gradient(90deg, #1d4e89, #3ee0e0, #ffb020);
	}

	.swatch.here {
		width: 10px;
		height: 10px;
		border: 2px solid #fff;
		border-radius: 50%;
		background: transparent;
	}

	@media (min-width: 640px) {
		.container {
			padding: 20px;
			gap: 10px;
		}

		h1 {
			font-size: 1.5rem;
		}

		.subtitle {
			font-size: 1rem;
		}
	}
</style>
