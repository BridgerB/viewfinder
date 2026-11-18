<script lang="ts">
import { onDestroy, onMount, tick } from "svelte";
import LocateMap from "#lib/LocateMap.svelte";
import {
	API,
	type Box,
	dragBox,
	fitBox,
	type Grab,
	grabAt,
	grabCursor,
	type Located,
	type Point,
	parseFov,
	svgPoints,
	type Traced,
} from "#lib/locate.ts";
import ResultSummary from "#lib/ResultSummary.svelte";
import { dev } from "$app/env";
import { replaceState } from "$app/navigation";
import { resolve } from "$app/paths";

/** Where the locator server stands: unknown until the first check. */
let server = $state<"checking" | "up" | "down">("checking");
let places = $state(0);

let preview = $state("");
let traced = $state<Traced | null>(null);
let imageUrl = $state("");
let phase = $state<
	"idle" | "uploading" | "confirm" | "retracing" | "searching" | "done"
>("idle");
/** The skyline a search will use: the whole-photo trace, or the trace inside `box`. */
let skyline = $state<Point[]>([]);
/** The box the skyline was traced in, or null for the whole photo. */
let box = $state<Box | null>(null);
/** The box while it is being dragged, and until the retrace it started finishes. */
let liveBox = $state<Box | null>(null);
let cursor = $state("crosshair");
let fovText = $state("");
let editingFov = $state(false);
let error = $state("");
let result = $state<Located | null>(null);
let shareId = $state("");
let shareNote = $state("");
let elapsed = $state(0);
let dragging = $state(false);
let linkCopied = $state(false);
let previewBroken = $state(false);
let panel: HTMLElement | undefined;
let timer: ReturnType<typeof setInterval> | undefined;
/** The drag in progress: what was grabbed, where, and the box when it started. */
let drag: { grab: Grab; from: Point; start: Box } | null = null;

const busy = $derived(
	phase === "uploading" || phase === "retracing" || phase === "searching",
);
/** The pink box: the one the skyline was traced in, or one fitted around the whole-photo skyline. */
const fitted = $derived(
	traced ? (box ?? fitBox(skyline, traced.width, traced.height)) : null,
);
const shownBox = $derived(liveBox ?? (phase === "uploading" ? null : fitted));
/** The box can be dragged whenever nothing is running. */
const editable = $derived(phase === "done" || phase === "confirm");
const shareUrl = $derived(
	shareId && typeof window !== "undefined"
		? `${window.location.origin}${resolve("/p/[id]", { id: shareId })}`
		: "",
);
const lensText = $derived(
	parseFov(fovText)
		? `${fovText}°${traced?.fov ? ", from the photo" : ""}`
		: "unknown, tries 40–115°",
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

function post<T>(path: string, payload: unknown): Promise<T> {
	return call<T>(path, {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(payload),
	});
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

// A running clock while the locator works, so a search never looks stuck.
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
	shareId = "";
	shareNote = "";
	skyline = [];
	box = null;
	liveBox = null;
	fovText = "";
	editingFov = false;
	error = "";
	phase = "idle";
	if (window.location.pathname !== resolve("/locate"))
		void replaceState(resolve("/locate"), {});
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
		skyline = t.skyline;
		imageUrl = `${API}/image/${t.id}`;
		fovText = t.fov ? t.fov.toFixed(1) : "";
	} catch (e) {
		error = explain(e);
		phase = "idle";
		clearInterval(timer);
		return;
	}
	// No confirmation when the match is good: the result asks only if it is weak.
	await search();
};

/** Retrace inside `next`, or over the whole photo when it is null, then search again. */
const retrace = async (next: Box | null) => {
	if (!traced) return;
	error = "";
	phase = "retracing";
	startClock();
	try {
		const r = await post<{ skyline: Point[] }>(
			"/retrace",
			next ? { id: traced.id, crop: next.map(Math.round) } : { id: traced.id },
		);
		skyline = r.skyline;
		box = next;
	} catch (e) {
		error = explain(e);
		phase = result ? "done" : "confirm";
		clearInterval(timer);
		return;
	} finally {
		liveBox = null;
	}
	await search();
};

/**
 * Saves the result for sharing and puts its link in the address bar, without leaving the page.
 * The search already succeeded, so a failure here only costs the link.
 */
const save = async (found: Located) => {
	if (!traced) return;
	shareNote = "";
	try {
		const res = await fetch(resolve("/api/shares"), {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				upload: traced.id,
				width: traced.width,
				height: traced.height,
				skyline,
				box,
				result: found,
			}),
		});
		if (!res.ok) throw new Error(await res.text());
		shareId = ((await res.json()) as { id: string }).id;
		await replaceState(resolve("/p/[id]", { id: shareId }), {});
	} catch {
		shareId = "";
		shareNote = "Couldn't save a share link for this result.";
	}
};

const search = async () => {
	if (!traced) return;
	error = "";
	phase = "searching";
	startClock();
	try {
		const found = await post<Located>("/locate", {
			id: traced.id,
			fov: parseFov(fovText),
		});
		result = found;
		phase = "done";
		void save(found);
		await tick();
		panel?.scrollIntoView({ behavior: "smooth", block: "nearest" });
	} catch (e) {
		error = explain(e);
		phase = "confirm";
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

/** The photo's rectangle on screen; the drag surface is larger, so pointers map through this. */
const photoRect = (el: Element) =>
	(el.parentElement?.querySelector("img") ?? el).getBoundingClientRect();

/** A pointer position in image pixels, clamped to the photo. */
const toImage = (event: PointerEvent, el: Element): Point => {
	const r = photoRect(el);
	const w = traced?.width ?? 1;
	const h = traced?.height ?? 1;
	const x = ((event.clientX - r.left) * w) / r.width;
	const y = ((event.clientY - r.top) * h) / r.height;
	return [Math.min(w, Math.max(0, x)), Math.min(h, Math.max(0, y))];
};

/** 14 screen pixels of slack around the box's edges, in image pixels. */
const reach = (el: Element) =>
	(14 * (traced?.width ?? 1)) / photoRect(el).width;

const press = (event: PointerEvent & { currentTarget: HTMLButtonElement }) => {
	const el = event.currentTarget;
	el.setPointerCapture(event.pointerId);
	const p = toImage(event, el);
	const current = shownBox;
	const grab = current ? grabAt(current, p, reach(el)) : null;
	// On the box: move or resize it. Outside it: draw a new one from this corner.
	drag =
		current && grab
			? { grab, from: p, start: current }
			: { grab: "se", from: p, start: [p[0], p[1], p[0], p[1]] };
};

const move = (event: PointerEvent & { currentTarget: HTMLButtonElement }) => {
	const el = event.currentTarget;
	const p = toImage(event, el);
	if (!drag) {
		cursor = grabCursor(shownBox ? grabAt(shownBox, p, reach(el)) : null);
		return;
	}
	const w = traced?.width ?? 1;
	const h = traced?.height ?? 1;
	if (drag.start[2] - drag.start[0] < 1) {
		// Drawing a new box: span from the press point, either direction.
		const [ax, ay] = drag.from;
		liveBox = [
			Math.min(ax, p[0]),
			Math.min(ay, p[1]),
			Math.max(ax, p[0]),
			Math.max(ay, p[1]),
		];
	} else liveBox = dragBox(drag.start, drag.grab, drag.from, p, w, h);
};

const release = () => {
	const was = drag;
	drag = null;
	const next = liveBox;
	if (!was || !next) return;
	const changed = next.some((v, i) => Math.abs(v - was.start[i]) > 2);
	// A click without a drag changes nothing; a real change retraces and searches again.
	if (changed && next[2] - next[0] >= 16 && next[3] - next[1] >= 16)
		void retrace(next);
	else liveBox = null;
};

const copyLink = async () => {
	await navigator.clipboard.writeText(shareUrl);
	linkCopied = true;
	setTimeout(() => {
		linkCopied = false;
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
					<p class="dim">Searches {places.toLocaleString()} places in a few seconds.</p>
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
							{#if shownBox}
								{@const [x0, y0, x1, y1] = shownBox}
								<path
									class="shade"
									fill-rule="evenodd"
									d="M0 0H{traced.width}V{traced.height}H0Z M{x0} {y0}V{y1}H{x1}V{y0}Z"
								/>
								<rect class="boxline" x={x0} y={y0} width={x1 - x0} height={y1 - y0} />
								{#if editable}
									{@const r = traced.width / 140}
									{#each [[x0, y0], [(x0 + x1) / 2, y0], [x1, y0], [x1, (y0 + y1) / 2], [x1, y1], [(x0 + x1) / 2, y1], [x0, y1], [x0, (y0 + y1) / 2]] as [hx, hy], i (i)}
										<rect class="handle" x={hx - r} y={hy - r} width={r * 2} height={r * 2} />
									{/each}
								{/if}
							{/if}
							<polyline class="auto" points={svgPoints(skyline)} />
							{#if result && phase === "done"}
								<polyline class="horizon" points={svgPoints(result.outline)} />
							{/if}
						</svg>
					{/if}
					{#if editable && traced}
						<button
							type="button"
							class="canvas"
							style:cursor
							onpointerdown={press}
							onpointermove={move}
							onpointerup={release}
							onpointercancel={release}
							aria-label="Drag the pink box's edges to fit just the mountain ridge"
						></button>
						{#if shownBox && !liveBox}
							<!-- Pinned to the box's top-left corner, so it says what the box is for. -->
							<div
								class="boxtag"
								style:left="{(shownBox[0] / traced.width) * 100}%"
								style:top="{(shownBox[1] / traced.height) * 100}%"
							>
								⤡ Drag the edges to fit just the mountains
							</div>
						{/if}
					{/if}
					{#if busy}
						<div class="veil">
							<span class="spinner"></span>
							<span
								>{phase === "uploading"
									? "Tracing the skyline"
									: phase === "retracing"
										? "Tracing inside your box"
										: "Searching every place"}… {elapsed.toFixed(1)} s</span
							>
						</div>
					{/if}
				</div>
				<div class="legend">
					<span class="key auto">— skyline the search uses</span>
					{#if shownBox}<span class="key box">▭ mountains it read: drag the box to change</span>{/if}
					{#if result && phase === "done"}<span class="key horizon">— terrain horizon from the best place</span
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
				{:else if phase === "confirm"}
					<h2>The search couldn't use this skyline</h2>
					<p class="dim">
						The orange line should follow where the mountains meet the sky, not trees, roofs, poles
						or clouds. Drag the pink box's edges on the photo so it holds just the mountain ridge; the
						search runs again when you let go.
					</p>
					<div class="row">
						<button type="button" class="ghost" onclick={search}>Search again</button>
						{#if box}
							<button type="button" class="ghost" onclick={() => retrace(null)}
								>Use the whole photo</button
							>
						{/if}
					</div>
				{:else if phase === "retracing"}
					<ol class="steps">
						<li class="now">Tracing the ridge inside your box</li>
						<li>Searching about 32,000 places</li>
					</ol>
				{:else if phase === "searching"}
					<ol class="steps">
						<li class="ok">Skyline traced{box ? " inside your box" : ""}</li>
						<li class="now">
							Searching about 32,000 places, then refining the best at 250 m and 100 m
						</li>
					</ol>
				{:else if result}
					<ResultSummary {result} height={traced?.height ?? 1}>
						{#snippet afterVerdict(level)}
							{#if level !== "good"}
								<!-- Only a weak match asks for help; a good one needs no confirmation. -->
								<div class="nudge">
									<strong>Is the orange line on the mountain ridge?</strong>
									If it follows trees, roofs, poles or clouds, drag the pink box's edges on the photo in
									to just the mountains. The search runs again when you let go.
								</div>
							{/if}
						{/snippet}
					</ResultSummary>
					{#if box}
						<div class="row">
							<button type="button" class="ghost" onclick={() => retrace(null)}
								>Use the whole photo again</button
							>
						</div>
					{/if}
					<div class="sharebox">
						{#if shareId}
							<button type="button" class="go" onclick={copyLink}
								>{linkCopied ? "Link copied" : "Copy share link"}</button
							>
							<p class="dim">Anyone with the link sees this result. It isn't listed anywhere.</p>
						{:else if shareNote}
							<p class="dim">{shareNote}</p>
						{:else}
							<p class="dim">Saving a share link…</p>
						{/if}
					</div>
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
							<button type="button" class="ghost" onclick={search}>Search again</button>
						{:else}
							<span class="dim">Lens: {lensText}</span>
							<button type="button" class="link" onclick={() => (editingFov = true)}>change</button>
						{/if}
					</div>
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
	button {
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

	button:hover:not(:disabled) {
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
	.veil {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}

	.frame svg {
		pointer-events: none;
	}

	/* The drag surface: square (rounded corners would not take clicks) and 14 px larger than
	   the photo on every side, so a box edge or corner on the photo's border can be grabbed. */
	.canvas {
		position: absolute;
		inset: -14px;
		padding: 0;
		border: 0;
		background: transparent;
		cursor: crosshair;
		touch-action: none;
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

	/* Everything outside the mountain box is dimmed; the box gets a dashed outline. */
	.shade {
		fill: #0a0a0f99;
	}


	.handle {
		fill: #ff5cc8;
		stroke: #fff;
		stroke-width: 1.5px;
		vector-effect: non-scaling-stroke;
		animation: beckon 1.6s ease-in-out 3;
	}

	@keyframes beckon {
		50% {
			opacity: 0.35;
		}
	}

	.boxtag {
		position: absolute;
		padding: 4px 8px;
		border-radius: 0 0 6px 0;
		background: #ff5cc8;
		color: #0a0a0f;
		font-size: 0.75rem;
		font-weight: 600;
		line-height: 1.3;
		white-space: nowrap;
		pointer-events: none;
	}

	@media (prefers-reduced-motion: reduce) {
		.handle {
			animation: none;
		}
	}

	.sharebox {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding-top: 10px;
		border-top: 1px solid #222;
	}

	.boxline {
		fill: none;
		stroke: #ff5cc8;
		stroke-width: 2px;
		stroke-dasharray: 8 5;
		vector-effect: non-scaling-stroke;
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

	.link {
		padding: 0 2px;
		border: 0;
		background: none;
		color: #4facfe;
		text-decoration: underline;
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
