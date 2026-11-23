<script lang="ts">
import example from "#lib/assets/sandy-example.json";
import LocateMap from "#lib/LocateMap.svelte";
import { fitBox, type Located, type Point } from "#lib/locate.ts";
import PhotoFrame from "#lib/PhotoFrame.svelte";
import { resolve } from "$app/paths";

// A real search on the Sandy sample photo (static/sandy.jpg), saved from `sight serve`.
const result = example.result as unknown as Located;
const skyline = example.skyline as unknown as Point[];
const box = fitBox(skyline, example.width, example.height);
const frame = {
	src: "/sandy.jpg",
	width: example.width,
	height: example.height,
};
</script>

<svelte:head>
	<title>How it works · Viewfinder</title>
	<meta
		name="description"
		content="Viewfinder traces the skyline in your photo, compares it with the horizon from every spot in the terrain, and pins the best match."
	/>
</svelte:head>

<section class="intro">
	<div class="eyebrow">How it works</div>
	<h1>Match the ridge.</h1>
	<p>
		Every spot on the ground sees its own skyline. Viewfinder computes that skyline for about 32,000
		spots from USGS elevation data, then finds the one that matches your photo.
	</p>
</section>

<ol class="steps">
	<li>
		<PhotoFrame {...frame} alt="The Sandy sample photo with its skyline traced in yellow" {skyline} />
		<div class="caption">
			<span class="n">1</span>
			<span><b>Trace</b> your skyline. The yellow line follows where the mountains meet the sky.</span>
		</div>
	</li>
	<li>
		<PhotoFrame
			{...frame}
			alt="The same photo with the terrain horizon in cyan lying on the traced skyline"
			{skyline}
			horizon={result.outline}
		/>
		<div class="caption">
			<span class="n">2</span>
			<span
				><b>Compare</b> it with terrain from every spot, at every heading and lens. The cyan line is the
				horizon from the best one.</span
			>
		</div>
	</li>
	<li>
		<LocateMap {result} heat="#2f7d4a" />
		<div class="caption">
			<span class="n">3</span>
			<span
				><b>Pin</b> the best match. The wedge shows which way the camera faced, out to the ridge in the
				photo.</span
			>
		</div>
	</li>
	<li>
		<PhotoFrame
			{...frame}
			alt="The photo with a pink box around the ridge"
			{skyline}
			{box}
			boxMode="edit"
		/>
		<div class="caption">
			<span class="n">4</span>
			<span
				><b>Adjust</b> the box to leave out trees, roofs or clouds. The search runs again when you let
				go.</span
			>
		</div>
	</li>
</ol>

<div class="footer">
	<span>Accurate to a few hundred metres · Salt Lake, Utah and Davis counties · needs a skyline</span>
	<a class="pill big" href={resolve("/")}>Try a photo</a>
</div>

<style>
	.intro {
		display: grid;
		gap: 10px;
		padding: 48px 0 16px;
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

	.steps {
		display: grid;
		gap: 44px;
		margin: 0;
		padding: 16px 0 0;
		list-style: none;
	}

	.steps li {
		display: grid;
		gap: 12px;
	}

	.caption {
		display: flex;
		gap: 14px;
		align-items: baseline;
		max-width: 62ch;
	}

	.n {
		color: var(--accent);
		font: 400 36px / 1 var(--serif);
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
