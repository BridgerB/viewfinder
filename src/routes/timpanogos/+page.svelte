<script lang="ts">
	import viewpointsData from '$lib/assets/timpanogos.json';

	interface Coordinate {
		latitude: number;
		longitude: number;
	}

	interface HorizonPoint {
		relativeDirection: number;
		elevationAngleDegrees: number;
		distanceKm: number;
	}

	interface Viewpoint {
		angle: number;
		viewpoint: Coordinate;
		bearingToPeak: number;
		horizon: HorizonPoint[];
	}

	interface RgbColor {
		red: number;
		green: number;
		blue: number;
	}

	const MAX_ELEVATION_DEGREES = 25;
	const MAX_DISTANCE_KM = 15;
	const CLOSE_COLOR: RgbColor = { red: 255, green: 107, blue: 53 };
	const FAR_COLOR: RgbColor = { red: 65, green: 88, blue: 208 };

	const clamp = (value: number, min: number, max: number): number =>
		Math.min(Math.max(value, min), max);

	const linearInterpolate = (start: number, end: number, ratio: number): number =>
		Math.round(start + (end - start) * ratio);

	const interpolateColor = (fromColor: RgbColor, toColor: RgbColor, ratio: number): RgbColor => ({
		red: linearInterpolate(fromColor.red, toColor.red, ratio),
		green: linearInterpolate(fromColor.green, toColor.green, ratio),
		blue: linearInterpolate(fromColor.blue, toColor.blue, ratio)
	});

	const rgbToString = (color: RgbColor): string => `rgb(${color.red},${color.green},${color.blue})`;

	const calculateDistanceColor = (distanceKm: number): string => {
		const interpolationRatio = clamp(distanceKm / MAX_DISTANCE_KM, 0, 1);
		return rgbToString(interpolateColor(CLOSE_COLOR, FAR_COLOR, interpolationRatio));
	};

	const calculateBarHeight = (elevationDegrees: number): number =>
		(elevationDegrees / MAX_ELEVATION_DEGREES) * 100;

	const formatCoordinate = (coordinate: Coordinate): string =>
		`${coordinate.latitude.toFixed(4)}°N, ${Math.abs(coordinate.longitude).toFixed(4)}°W`;

	let viewpoints: Viewpoint[] = viewpointsData as Viewpoint[];
	let currentAngle = $state(187);
	let playing = $state(true);
	let animationSpeed = $state(500);
	let animationInterval: ReturnType<typeof setInterval>;

	$effect(() => {
		if (!playing) return () => {};

		const intervalDelayMs = 500 - animationSpeed + 10;
		animationInterval = setInterval(() => {
			currentAngle = (currentAngle + 1) % 360;
		}, intervalDelayMs);

		return () => clearInterval(animationInterval);
	});

	const togglePlay = (): void => {
		playing = !playing;
	};
</script>

<div class="container">
	<h1>Mt Timpanogos - {currentAngle}°</h1>
	<p class="subtitle">{formatCoordinate(viewpoints[currentAngle].viewpoint)}</p>

	<div class="chart">
		{#each viewpoints[currentAngle].horizon as horizonPoint (horizonPoint.relativeDirection)}
			<div
				class="bar"
				style="--height: {calculateBarHeight(
					horizonPoint.elevationAngleDegrees
				)}%; --color: {calculateDistanceColor(horizonPoint.distanceKm)}"
				title="{horizonPoint.relativeDirection}° | {horizonPoint.elevationAngleDegrees.toFixed(
					1
				)}° | {horizonPoint.distanceKm.toFixed(1)}km"
			></div>
		{/each}
	</div>

	<div class="x-axis">
		<span>-45°</span>
		<span>Peak</span>
		<span>+45°</span>
	</div>

	<div class="controls">
		<button onclick={togglePlay}>
			{playing ? 'Pause' : 'Play'}
		</button>

		<input class="angle-slider" type="range" min="0" max="359" bind:value={currentAngle} />

		<span class="angle">{currentAngle}°</span>

		<label class="speed-control">
			Speed:
			<input class="speed-slider" type="range" min="10" max="500" bind:value={animationSpeed} />
		</label>
	</div>

	<p class="info">8.9km from Mt Timpanogos peak</p>
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
		min-height: 100vh;
		min-height: 100dvh;
		padding: 12px;
		box-sizing: border-box;
		gap: 8px;
	}

	h1 {
		margin: 0;
		text-align: center;
		font-size: 1.2rem;
	}

	.subtitle {
		text-align: center;
		color: #666;
		margin: 0;
		font-size: 0.85rem;
	}

	.chart {
		height: 40vh;
		display: flex;
		align-items: flex-end;
		gap: 1px;
		background: #111;
		padding: 12px;
		border-radius: 8px;
	}

	.bar {
		flex: 1;
		height: var(--height);
		background: var(--color);
		min-height: 2px;
	}

	.x-axis {
		display: flex;
		justify-content: space-between;
		padding: 4px 12px;
		color: #666;
		font-size: 11px;
	}

	.controls {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		align-items: center;
		gap: 12px;
		padding: 12px;
		background: #111;
		border-radius: 8px;
	}

	button {
		padding: 10px 20px;
		background: #4facfe;
		border: none;
		border-radius: 5px;
		color: #000;
		cursor: pointer;
		font-weight: bold;
		flex-shrink: 0;
	}

	button:hover {
		background: #00f2fe;
	}

	.angle-slider {
		flex: 1;
		min-width: 100px;
		max-width: 200px;
	}

	.angle {
		font-size: 1.2rem;
		font-weight: bold;
		color: #4facfe;
		min-width: 50px;
		text-align: center;
	}

	.speed-control {
		display: flex;
		align-items: center;
		gap: 8px;
		color: #888;
		font-size: 0.85rem;
	}

	.speed-slider {
		width: 80px;
	}

	.info {
		text-align: center;
		color: #666;
		margin: 0;
		font-size: 12px;
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
			margin-bottom: 10px;
		}

		.chart {
			height: 60vh;
			padding: 20px;
		}

		.x-axis {
			padding: 10px 20px;
			font-size: 12px;
		}

		.controls {
			padding: 20px;
			gap: 20px;
		}

		.angle {
			font-size: 1.5rem;
			min-width: 60px;
		}

		.angle-slider {
			max-width: 150px;
		}

		.speed-slider {
			width: 100px;
		}

		.info {
			font-size: 14px;
			margin-top: 5px;
		}
	}
</style>
