<script lang="ts">
	import { resolve } from '$app/paths';
	import viewpointsData from '$lib/assets/timpanogos.json';
	import matchData from '$lib/assets/timpanogos-match.json';
	import searchData from '$lib/assets/timpanogos-search.json';
	import * as photoMatch from '$lib/photo-match';
	import MatchMap from '$lib/MatchMap.svelte';

	const viewpoints = viewpointsData as photoMatch.Viewpoint[];
	const match = photoMatch.fromJson(matchData);
	const lens = photoMatch.lensOf(match.camera);
	const skylinePoints = photoMatch.skylinePoints(match.skyline);
	const scorePoints = photoMatch.scorePoints(match.correlation);
	const search = photoMatch.searchHeat(searchData as photoMatch.SearchHit[]);

	let currentAngle = $state(match.bestAngle);
	const angle = $derived(photoMatch.clampIndex(Number(currentAngle)));
	const viewpoint = $derived(photoMatch.viewpointAt(viewpoints, angle));
	const horizonPoints = $derived(photoMatch.horizonPoints(lens, viewpoint.horizon));
	const profile = $derived(photoMatch.profileOf(lens, match.skyline, viewpoint.horizon));

	const chooseAngle = (event: PointerEvent & { currentTarget: EventTarget & SVGSVGElement }) => {
		const bounds = event.currentTarget.getBoundingClientRect();
		currentAngle = photoMatch.angleAt((event.clientX - bounds.left) / bounds.width);
	};
</script>

<svelte:head>
	<title>Timpanogos photo match</title>
</svelte:head>

<div class="container">
	<p class="back"><a href={resolve('/timpanogos')}>Orbit</a></p>
	<h1>Photo match · {angle}°</h1>
	<p class="subtitle">
		{photoMatch.percentText(photoMatch.shapeMatch(match, angle))} shape match · {photoMatch
			.averageError(match, angle)
			.toFixed(2)}° average error
	</p>
	<p class="subtitle">{photoMatch.coordinateText(viewpoint.viewpoint)}</p>

	<div class="frame">
		<img src="/mount-timpanogos.jpg" alt="Photograph of Mount Timpanogos from the valley" />
		<svg
			viewBox="0 0 {match.camera.imageWidth} {match.camera.imageHeight}"
			preserveAspectRatio="xMidYMid meet"
		>
			<polyline class="skyline" points={skylinePoints} />
			<polyline class="model" points={horizonPoints} />
		</svg>
	</div>

	<ul class="legend">
		<li><span class="swatch skyline"></span> Skyline traced from the photo</li>
		<li><span class="swatch model"></span> Terrain horizon at this angle</li>
	</ul>

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

	<h2>Shape match around the ring</h2>
	<svg
		class="score"
		viewBox="0 0 359 100"
		preserveAspectRatio="none"
		role="img"
		aria-label="Shape match for each viewpoint angle"
		onpointerdown={chooseAngle}
	>
		<polygon points="0,100 {scorePoints} 359,100" fill="rgba(62, 224, 224, 0.18)" stroke="none" />
		<polyline
			points={scorePoints}
			fill="none"
			stroke="#3ee0e0"
			stroke-width="1.5"
			vector-effect="non-scaling-stroke"
		/>
		<line
			x1={angle}
			y1="0"
			x2={angle}
			y2="100"
			stroke="#fff"
			stroke-width="1.5"
			vector-effect="non-scaling-stroke"
		/>
	</svg>
	<div class="axis">
		<span>0°</span>
		<span>Best {match.bestAngle}°</span>
		<span>359°</span>
	</div>

	<h2>Elevation angle</h2>
	<svg
		class="profile"
		viewBox="0 0 100 100"
		preserveAspectRatio="none"
		role="img"
		aria-label="Photo skyline and terrain horizon as elevation angles"
	>
		<polyline points={profile.photo} class="skyline" />
		<polyline points={profile.model} class="model" />
	</svg>
	<div class="axis">
		<span
			>{photoMatch.signedDegrees(profile.azimuth.min)} to {photoMatch.signedDegrees(
				profile.azimuth.max
			)} from the peak</span
		>
		<span>{profile.elevation.min.toFixed(0)}° to {profile.elevation.max.toFixed(0)}° elevation</span
		>
	</div>

	<div class="controls">
		<label>
			Viewpoint
			<input type="range" min="0" max="359" bind:value={currentAngle} />
		</label>
		<span class="angle">{angle}°</span>
		<button type="button" onclick={() => (currentAngle = match.bestAngle)}>Best match</button>
	</div>

	<p class="info">
		The photograph lines up with the {match.bestAngle}° viewpoint on the 8.9 km ring:
		{photoMatch.percentText(photoMatch.shapeMatch(match, match.bestAngle))} shape match,
		{photoMatch.averageError(match, match.bestAngle).toFixed(2)}° average error. The fit uses a {match
			.camera.fovH}° horizontal field of view, tilted {match.camera.pitch.toFixed(1)}° up and rolled
		{match.camera.roll.toFixed(0)}°. The summit sits {Math.abs(match.camera.centerAzimuth).toFixed(
			0
		)}° left of the frame center. The edges disagree more than the crest, which is where a phone
		lens stops being a perfect rectilinear camera.
	</p>
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

	.model {
		stroke: #3ee0e0;
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

	.score {
		width: 100%;
		height: 140px;
		background: #111;
		border-radius: 8px;
		cursor: pointer;
		touch-action: none;
	}

	.profile {
		width: 100%;
		height: 180px;
		background: #111;
		border-radius: 8px;
	}

	.axis {
		display: flex;
		justify-content: space-between;
		color: #666;
		font-size: 11px;
	}

	.controls {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: center;
		gap: 12px;
		padding: 12px;
		background: #111;
		border-radius: 8px;
	}

	label {
		display: flex;
		flex: 1;
		align-items: center;
		gap: 8px;
		min-width: 180px;
		color: #aaa;
		font-size: 0.85rem;
	}

	input[type='range'] {
		flex: 1;
	}

	.angle {
		min-width: 52px;
		color: #4facfe;
		font-size: 1.2rem;
		font-weight: bold;
		text-align: center;
	}

	button {
		padding: 10px 16px;
		border: none;
		border-radius: 5px;
		background: #4facfe;
		color: #000;
		font-weight: bold;
		cursor: pointer;
	}

	button:hover {
		background: #00f2fe;
	}

	.info {
		margin: 0;
		color: #888;
		font-size: 0.85rem;
		line-height: 1.45;
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
