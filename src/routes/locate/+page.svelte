<script lang="ts">
import { onDestroy, onMount, tick } from "svelte";
import LocateMap from "#lib/LocateMap.svelte";
import {
	API,
	kmBetween,
	type Located,
	type Point,
	parseFov,
	spreadKm,
	svgPoints,
	type Traced,
	verdict,
} from "#lib/locate.ts";
import { dev } from "$app/env";
import { resolve } from "$app/paths";

/** Where the locator server stands: unknown until the first check. */
let server = $state<"checking" | "up" | "down">("checking");
let places = $state(0);

let preview = $state("");
let traced = $state<Traced | null>(null);
let imageUrl = $state("");
let phase = $state<"idle" | "uploading" | "searching" | "drawing" | "done">(
	"idle",
);
let drawn = $state<Point[]>([]);
/** The skyline the last search used: the traced line or the drawn one. */
let usedDrawn = $state(false);
let fovText = $state("");
let editingFov = $state(false);
let error = $state("");
let result = $state<Located | null>(null);
let elapsed = $state(0);
let dragging = $state(false);
let copied = $state(false);
let previewBroken = $state(false);
let panel: HTMLElement | undefined;
let timer: ReturnType<typeof setInterval> | undefined;

const drawnInOrder = $derived(drawn.toSorted((a, b) => a[0] - b[0]));
const spread = $derived(result ? spreadKm(result.top) : 0);
const judged = $derived(result ? verdict(result.best, spread) : null);
const busy = $derived(phase === "uploading" || phase === "searching");
const coords = $derived(
	result ? `${result.best.lat.toFixed(5)}, ${result.best.lon.toFixed(5)}` : "",
);

const explain = (e: unknown) => {
	if (e instanceof TypeError) {
		server = "down";
		return dev
			? "The locator stopped answering. Start it again with: pnpm sight"
			: "The locator stopped answering. Try again in a moment.";
	}
	return e instanceof Error ? e.message : String(e);
};

async function call<T>(path: string, init?: RequestInit): Promise<T> {
	const res = await fetch(`${API}${path}`, init);
	// A locator error is JSON with an `error`; a failed start or proxy error may be plain text.
	const body = (await res.json().catch(() => ({}))) as T & { error?: string };
	if (!res.ok)
		throw new Error(
			body.error ?? `The locator answered ${res.status} ${res.statusText}`,
		);
	return body;
}

const checkServer = async () => {
	server = "checking";
	try {
		places = (await call<{ places: number }>("/health")).places;
		server = "up";
	} catch {
		server = "down";
	}
};

onMount(() => {
	void checkServer();
});

// A running clock while the locator works, so a 2-second search never looks stuck.
const startClock = () => {
	clearInterval(timer);
	const started = performance.now();
	elapsed = 0;
	timer = setInterval(() => {
		elapsed = (performance.now() - started) / 1000;
	}, 100);
};

onDestroy(() => clearInterval(timer));

const reset = () => {
	if (preview) URL.revokeObjectURL(preview);
	preview = "";
	previewBroken = false;
	traced = null;
	imageUrl = "";
	result = null;
	drawn = [];
	usedDrawn = false;
	fovText = "";
	editingFov = false;
	error = "";
	phase = "idle";
};

const start = async (file: Blob) => {
	reset();
	// Browsers show JPEG and PNG straight away; HEIC waits for the locator's upright copy.
	preview = URL.createObjectURL(file);
	phase = "uploading";
	startClock();
	try {
		const t = await call<Traced>("/trace", { method: "POST", body: file });
		traced = t;
		imageUrl = `${API}/image/${t.id}`;
		fovText = t.fov ? t.fov.toFixed(1) : "";
		await search(false);
	} catch (e) {
		error = explain(e);
		phase = traced ? "done" : "idle";
		clearInterval(timer);
	}
};

const search = async (withDrawn: boolean) => {
	if (!traced) return;
	error = "";
	phase = "searching";
	startClock();
	try {
		result = await call<Located>("/locate", {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				id: traced.id,
				skyline: withDrawn ? drawnInOrder : undefined,
				fov: parseFov(fovText),
			}),
		});
		usedDrawn = withDrawn;
		phase = "done";
		await tick();
		panel?.scrollIntoView({ behavior: "smooth", block: "nearest" });
	} catch (e) {
		error = explain(e);
		phase = withDrawn ? "drawing" : "done";
	} finally {
		clearInterval(timer);
	}
};

const pick = (event: Event & { currentTarget: HTMLInputElement }) => {
	const file = event.currentTarget.files?.[0];
	event.currentTarget.value = "";
	if (file) void start(file);
};

const drop = (event: DragEvent) => {
	event.preventDefault();
	dragging = false;
	const file = event.dataTransfer?.files[0];
	if (file && server === "up") void start(file);
};

const sample = async () => {
	const res = await fetch("/sandy.jpg");
	void start(await res.blob());
};

const addPoint = (event: MouseEvent & { currentTarget: HTMLButtonElement }) => {
	// A keyboard "click" has no position; only pointer clicks place points.
	if (!traced || event.detail === 0) return;
	const box = event.currentTarget.getBoundingClientRect();
	drawn = [
		...drawn,
		[
			((event.clientX - box.left) * traced.width) / box.width,
			((event.clientY - box.top) * traced.height) / box.height,
		],
	];
};

const copy = async () => {
	await navigator.clipboard.writeText(coords);
	copied = true;
	setTimeout(() => {
		copied = false;
	}, 1500);
};
</script>

<svelte:head>
	<title>Locate a photo</title>
</svelte:head>

<div class="page">
	<header>
		<a class="back" href={resolve("/")}>← Home</a>
		<h1>Where was this photo taken?</h1>
		{#if phase !== "idle"}
			<button type="button" class="ghost" onclick={reset} disabled={busy}>New photo</button>
		{/if}
	</header>

	{#if server === "down"}
		<div class="banner bad">
			<div>
				{#if dev}
					<strong>The locator isn't running.</strong> Start it in a terminal from the project folder:
					<code>pnpm sight</code>
				{:else}
					<strong>The locator is unavailable right now.</strong> It may still be starting; try again
					in a few seconds.
				{/if}
			</div>
			<button type="button" class="ghost" onclick={checkServer}>Check again</button>
		</div>
	{/if}

	{#if phase === "idle"}
		<label
			class={["drop", { dragging, off: server !== "up" }]}
			ondragover={(e) => {
				e.preventDefault();
				dragging = true;
			}}
			ondragleave={() => (dragging = false)}
			ondrop={drop}
		>
			<input
				type="file"
				accept="image/*,.heic,.heif"
				onchange={pick}
				disabled={server !== "up"}
			/>
			<span class="big">Drop a photo here, or click to choose one</span>
			<span class="small">JPEG, PNG or iPhone HEIC</span>
		</label>
		{#if error}<p class="error">{error}</p>{/if}
		<div class="tips">
			<div>
				<h2>Works best with</h2>
				<ul>
					<li>A clear mountain ridge against the sky, in Salt Lake or Utah County</li>
					<li>A level, landscape shot that shows a good stretch of skyline</li>
					<li>The original file, so the lens is read from it</li>
				</ul>
			</div>
			<div>
				<h2>No photo handy?</h2>
				<button type="button" class="ghost" onclick={sample} disabled={server !== "up"}
					>Try the Sandy sample</button
				>
				{#if server === "up"}
					<p class="dim">Searches {places.toLocaleString()} places in about 2 seconds.</p>
				{:else if server === "checking"}
					<p class="dim">Starting the locator… the first visit after a while takes a few seconds.</p>
				{/if}
			</div>
		</div>
	{:else}
		<div class="work">
			<section class="stage">
				<div class="frame">
					<img
						src={imageUrl || preview}
						alt="Your upload with its skyline drawn over it"
						class={{ hidden: previewBroken }}
						onerror={() => {
							// A HEIC preview the browser cannot show; the locator's JPEG replaces it.
							previewBroken = !imageUrl;
						}}
						onload={() => {
							previewBroken = false;
						}}
					/>
					{#if traced}
						<svg viewBox="0 0 {traced.width} {traced.height}" preserveAspectRatio="none">
							<polyline
								class={["auto", { faded: phase === "drawing" || usedDrawn }]}
								points={svgPoints(traced.skyline)}
							/>
							{#if (phase === "drawing" || usedDrawn) && drawnInOrder.length}
								<polyline class="drawn" points={svgPoints(drawnInOrder)} />
								{#each drawnInOrder as [x, y], i (i)}
									<circle class="dot" cx={x} cy={y} r={traced.width / 220} />
								{/each}
							{/if}
							{#if result && phase !== "drawing"}
								<polyline class="horizon" points={svgPoints(result.outline)} />
							{/if}
						</svg>
					{/if}
					{#if phase === "drawing"}
						<button
							type="button"
							class="canvas"
							onclick={addPoint}
							aria-label="Click along the ridge to add a point to your skyline"
						></button>
						<div class="coach">Click along the ridge line, left to right or in any order</div>
					{/if}
					{#if busy}
						<div class="veil">
							<span class="spinner"></span>
							<span
								>{phase === "uploading" ? "Tracing the skyline" : "Searching every place"}… {elapsed.toFixed(
									1,
								)} s</span
							>
						</div>
					{/if}
				</div>
				<div class="legend">
					{#if traced}<span class="key auto">— traced skyline</span>{/if}
					{#if phase === "drawing" || usedDrawn}<span class="key drawn">— your skyline</span>{/if}
					{#if result && phase !== "drawing"}<span class="key horizon">— terrain horizon from the best place</span
						>{/if}
				</div>
			</section>

			<aside
				class="side"
				{@attach (node) => {
					panel = node;
				}}
			>
				{#if phase === "uploading"}
					<ol class="steps">
						<li class="now">Reading the photo and tracing its skyline</li>
						<li>Searching about 32,000 places</li>
					</ol>
				{:else if phase === "searching"}
					<ol class="steps">
						<li class="ok">Skyline {usedDrawn || drawn.length ? "drawn" : "traced"}</li>
						<li class="now">
							Searching about 32,000 places, then refining the best at 250 m and 100 m
						</li>
					</ol>
				{:else if phase === "drawing"}
					<h2>Draw the skyline</h2>
					<p class="dim">
						Click where the ridge meets the sky. Skip trees, poles and roofs: gaps are fine. More points
						on bumps and peaks help most.
					</p>
					<p class="count">{drawn.length} {drawn.length === 1 ? "point" : "points"}</p>
					<div class="row">
						<button
							type="button"
							class="ghost"
							onclick={() => (drawn = drawn.slice(0, -1))}
							disabled={!drawn.length}>Undo</button
						>
						<button type="button" class="ghost" onclick={() => (drawn = [])} disabled={!drawn.length}
							>Clear</button
						>
						<button type="button" class="ghost" onclick={() => (phase = "done")}>Cancel</button>
					</div>
					<button
						type="button"
						class="go"
						onclick={() => search(true)}
						disabled={drawn.length < 2}>Search with my skyline</button
					>
				{:else if result && judged}
					<div class={["verdict", judged.level]}>{judged.text}</div>
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
					{#if judged.level !== "good" && !usedDrawn}
						<div class="nudge">
							Does the orange line follow the ridge? If it rides on trees or roofs, draw it yourself.
							<button type="button" class="go" onclick={() => (phase = "drawing")}
								>Draw the skyline</button
							>
						</div>
					{:else}
						<div class="row">
							<button type="button" class="ghost" onclick={() => (phase = "drawing")}
								>{usedDrawn ? "Edit my skyline" : "Draw the skyline instead"}</button
							>
						</div>
					{/if}
					<details class="more">
						<summary>Lens and candidates</summary>
						<div class="lens">
							{#if editingFov}
								<label
									>Lens width <input
										type="text"
										inputmode="decimal"
										bind:value={fovText}
										placeholder="any"
										size="5"
									/> °</label
								>
								<button type="button" class="ghost" onclick={() => search(usedDrawn)}>Search again</button>
							{:else}
								<span
									>Lens: {parseFov(fovText) ? `${fovText}°, from the photo` : "unknown, tried 40–115°"}</span
								>
								<button type="button" class="ghost" onclick={() => (editingFov = true)}>Change</button>
							{/if}
						</div>
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
				{/if}
				{#if error}<p class="error">{error}</p>{/if}
			</aside>
		</div>

		<section class="mapbox">
			{#if result}
				<LocateMap {result} />
				{#if phase === "searching"}<div class="veil soft">Updating…</div>{/if}
			{:else}
				<div class="placeholder">
					{busy ? "The map appears when the search finishes." : "No result yet."}
				</div>
			{/if}
		</section>
	{/if}
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

	h2 {
		margin: 0 0 6px;
		font-size: 0.95rem;
	}

	a {
		color: #4facfe;
	}

	.back {
		font-size: 0.85rem;
	}

	.dim {
		color: #8a8a96;
	}

	.error {
		margin: 0;
		color: #ff6b6b;
		font-size: 0.85rem;
	}

	code {
		padding: 1px 6px;
		border-radius: 4px;
		background: #000;
		font-size: 0.85rem;
	}

	/* Buttons */
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

	button:hover:not(:disabled),
	.button:hover {
		border-color: #555;
	}

	button:disabled {
		opacity: 0.45;
		cursor: default;
	}

	.go {
		width: 100%;
		padding: 10px 14px;
		border-color: #3ecf6e;
		background: #3ecf6e22;
		font-weight: 600;
	}

	.row {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	/* Start screen */
	.banner {
		display: flex;
		flex-wrap: wrap;
		gap: 12px;
		align-items: center;
		justify-content: space-between;
		padding: 10px 14px;
		border-radius: 8px;
		font-size: 0.88rem;
	}

	.banner.bad {
		border: 1px solid #ff6b6b;
		background: #ff6b6b1a;
	}

	.drop {
		display: flex;
		flex-direction: column;
		gap: 6px;
		align-items: center;
		justify-content: center;
		min-height: 260px;
		border: 2px dashed #333;
		border-radius: 12px;
		background: #101018;
		cursor: pointer;
		transition:
			border-color 0.15s,
			background 0.15s;
	}

	.drop:hover,
	.drop.dragging {
		border-color: #4facfe;
		background: #4facfe12;
	}

	.drop.off {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.drop input {
		position: absolute;
		width: 1px;
		height: 1px;
		opacity: 0;
	}

	.drop .big {
		font-size: 1.05rem;
		font-weight: 600;
	}

	.drop .small {
		color: #8a8a96;
		font-size: 0.82rem;
	}

	.tips {
		display: grid;
		grid-template-columns: 1fr;
		gap: 16px;
		font-size: 0.85rem;
	}

	.tips ul {
		margin: 0;
		padding-left: 18px;
		color: #bbb;
		line-height: 1.6;
	}

	.tips p {
		margin: 8px 0 0;
	}

	@media (min-width: 760px) {
		.tips {
			grid-template-columns: 2fr 1fr;
		}
	}

	/* Working layout: photo and side panel */
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
		min-height: 120px;
		border-radius: 10px;
		background: #15151d;
	}

	.frame img.hidden {
		visibility: hidden;
	}

	.frame svg,
	.canvas,
	.veil {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}

	.frame svg {
		pointer-events: none;
	}

	.canvas {
		padding: 0;
		border: 2px solid #ff5cc8;
		border-radius: 10px;
		background: transparent;
		cursor: crosshair;
	}

	.coach {
		position: absolute;
		top: 10px;
		left: 50%;
		padding: 6px 10px;
		border-radius: 6px;
		background: #000c;
		font-size: 0.8rem;
		line-height: 1.3;
		transform: translateX(-50%);
		pointer-events: none;
	}

	.veil {
		display: flex;
		gap: 10px;
		align-items: center;
		justify-content: center;
		border-radius: 10px;
		background: #0a0a0fb3;
		font-size: 0.9rem;
		line-height: 1.3;
	}

	.veil.soft {
		background: #0a0a0f80;
	}

	.spinner {
		width: 18px;
		height: 18px;
		border: 3px solid #ffffff33;
		border-top-color: #4facfe;
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
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

	.auto.faded {
		opacity: 0.3;
	}

	.drawn {
		stroke: #ff5cc8;
	}

	.dot {
		fill: #ff5cc8;
	}

	.horizon {
		stroke: #3ee0e0;
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

	.key.drawn {
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

	.side p {
		margin: 0;
	}

	.steps {
		margin: 0;
		padding-left: 0;
		list-style: none;
		line-height: 1.5;
	}

	.steps li {
		padding-left: 22px;
		color: #8a8a96;
	}

	.steps li.now {
		color: #fff;
	}

	.steps li.now::before {
		content: "●";
		margin-left: -22px;
		padding-right: 10px;
		color: #4facfe;
	}

	.steps li.ok {
		color: #3ecf6e;
	}

	.steps li.ok::before {
		content: "✓";
		margin-left: -22px;
		padding-right: 10px;
	}

	.count {
		font-weight: 600;
	}

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

	.facts {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 4px 14px;
		margin: 0;
	}

	.facts dt {
		color: #8a8a96;
	}

	.facts dd {
		margin: 0;
		font-variant-numeric: tabular-nums;
	}

	.nudge {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 10px 12px;
		border-radius: 8px;
		background: #ff5cc81a;
		line-height: 1.4;
	}

	.more summary {
		color: #8a8a96;
		cursor: pointer;
	}

	.more[open] summary {
		margin-bottom: 8px;
	}

	.lens {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		align-items: center;
		margin-bottom: 8px;
	}

	.lens input {
		padding: 6px 8px;
		border: 1px solid #333;
		border-radius: 6px;
		background: #000;
		color: inherit;
		font: inherit;
	}

	.top {
		margin: 0;
		padding-left: 20px;
		line-height: 1.6;
	}

	.top li span + span {
		margin-left: 6px;
	}

	/* Map */
	.mapbox {
		position: relative;
	}

	.placeholder {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 200px;
		border: 1px dashed #222;
		border-radius: 12px;
		color: #8a8a96;
		font-size: 0.85rem;
	}

	@media (prefers-reduced-motion: reduce) {
		.spinner {
			animation: none;
		}
	}
</style>
