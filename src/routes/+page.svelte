<script lang="ts">
import { onDestroy, onMount, tick } from "svelte";
import CoverageMap from "#lib/CoverageMap.svelte";
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
	type Traced,
} from "#lib/locate.ts";
import PhotoFrame from "#lib/PhotoFrame.svelte";
import ResultView from "#lib/ResultView.svelte";
import { dev } from "$app/env";
import { afterNavigate, replaceState } from "$app/navigation";
import { resolve } from "$app/paths";

/** Where the locator stands: unknown until the first check. */
let server = $state<"checking" | "up" | "down">("checking");

let preview = $state("");
let previewBroken = $state(false);
let traced = $state<Traced | null>(null);
let imageUrl = $state("");
/**
 * idle: the start screen. uploading, retracing, searching: the locator is working. confirm: the
 * trace exists but the search could not use it, so the person adjusts the box. done: a result.
 * failed: nothing to show but an error.
 */
let phase = $state<
	| "idle"
	| "uploading"
	| "retracing"
	| "searching"
	| "confirm"
	| "done"
	| "failed"
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
let result = $state<Located | null>(null);
let shareId = $state("");
let shareNote = $state("");
let elapsed = $state(0);
let dragging = $state(false);
/** What went wrong, shown as a whole screen when there is nothing else to show. */
let problem = $state<{
	code: string;
	title: string;
	body: string;
	action: "home" | "retry" | "box";
} | null>(null);
let timer: ReturnType<typeof setInterval> | undefined;
let lastFile: Blob | null = null;
/** The drag in progress: what was grabbed, where, and the box when it started. */
let drag: { grab: Grab; from: Point; start: Box } | null = null;

const busy = $derived(
	phase === "uploading" || phase === "retracing" || phase === "searching",
);
/** The pink box: the one the skyline was traced in, or one fitted around the whole-photo skyline. */
const fitted = $derived(
	traced && skyline.length > 0
		? (box ?? fitBox(skyline, traced.width, traced.height))
		: null,
);
const shownBox = $derived(liveBox ?? fitted);
/** The box can be dragged whenever nothing is running. */
const editable = $derived(phase === "done" || phase === "confirm");
const shareUrl = $derived(
	shareId && typeof window !== "undefined"
		? `${window.location.origin}${resolve("/p/[id]", { id: shareId })}`
		: "",
);
const lensKnown = $derived(parseFov(fovText) !== null);
/** Rough length of a search on the container, for the progress bar: ~6.5 s with the lens, ~9 s trying every lens. */
const expected = $derived(lensKnown ? 7 : 10);
const steps = $derived([
	{ name: "Photo ready", state: "done" },
	{
		name: box ? "Tracing inside your box" : "Tracing the skyline",
		state: phase === "searching" ? "done" : "now",
	},
	{
		name: "Searching every spot",
		state: phase === "searching" ? "now" : "next",
	},
]);
const progress = $derived(
	phase === "searching"
		? 15 + Math.min(80, (elapsed / expected) * 80)
		: Math.min(15, elapsed * 6),
);

const explain = (e: unknown, during: "upload" | "search") => {
	if (e instanceof TypeError) {
		server = "down";
		return {
			code: "No response",
			title: "Server isn’t answering.",
			body: dev
				? "The locator stopped answering. Start it again with pnpm sight, then try again."
				: "Nothing was saved. Try again in a few seconds.",
			action: "retry" as const,
		};
	}
	const message = e instanceof Error ? e.message : String(e);
	if (/expired/.test(message))
		return {
			code: "Upload expired",
			title: "That photo timed out.",
			body: "The server only keeps recent uploads. Choose the photo again.",
			action: "home" as const,
		};
	if (/larger than/.test(message))
		return {
			code: "Too large",
			title: "That file is too big.",
			body: `${message[0].toUpperCase()}${message.slice(1)}. Try a smaller copy of the photo.`,
			action: "home" as const,
		};
	if (during === "upload")
		return {
			code: "Unsupported file",
			title: "Can’t read that file.",
			body: "Use a JPEG, PNG or HEIC photo.",
			action: "home" as const,
		};
	return {
		code: "No skyline",
		title: "No ridge found.",
		body: "Drag the pink box around where the mountains meet the sky, or use a photo with mountains against the sky.",
		action: "box" as const,
	};
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
		await call("/health");
		server = "up";
	} catch {
		server = "down";
	}
};

onMount(() => {
	void checkServer();
});

// "New photo" in the header links here; on this page it starts over.
afterNavigate((navigation) => {
	if (navigation.type === "link" && phase !== "idle") reset();
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

const stopClock = () => clearInterval(timer);

onDestroy(stopClock);

const reset = () => {
	stopClock();
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
	problem = null;
	phase = "idle";
	if (window.location.pathname !== resolve("/"))
		void replaceState(resolve("/"), {});
	window.scrollTo(0, 0);
};

const start = async (file: Blob) => {
	reset();
	lastFile = file;
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
		stopClock();
		problem = explain(e, "upload");
		phase = "failed";
		return;
	}
	// No confirmation when the match is good: the result asks only if it is weak.
	await search();
};

const retry = () => {
	if (traced) void search();
	else if (lastFile) void start(lastFile);
	else reset();
};

/** Retrace inside `next`, or over the whole photo when it is null, then search again. */
const retrace = async (next: Box | null) => {
	if (!traced) return;
	problem = null;
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
		stopClock();
		problem = explain(e, "search");
		phase = result ? "done" : "confirm";
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
	shareNote = "Saving a link…";
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
		shareNote = "";
		await replaceState(resolve("/p/[id]", { id: shareId }), {});
	} catch {
		shareId = "";
		shareNote = "Couldn’t save a share link for this result.";
	}
};

const search = async () => {
	if (!traced) return;
	problem = null;
	phase = "searching";
	startClock();
	try {
		const found = await post<Located>("/locate", {
			id: traced.id,
			fov: parseFov(fovText),
		});
		const first = !result;
		result = found;
		phase = "done";
		shareId = "";
		void save(found);
		if (first) {
			await tick();
			window.scrollTo({ top: 0 });
		}
	} catch (e) {
		problem = explain(e, "search");
		phase = result ? "done" : problem.action === "retry" ? "failed" : "confirm";
	} finally {
		stopClock();
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
	if (file) void start(file);
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

/** 16 screen pixels of slack around the box's edges, in image pixels. */
const reach = (el: Element) =>
	(16 * (traced?.width ?? 1)) / photoRect(el).width;

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
</script>

<svelte:head>
	<title>Viewfinder · Where was this photo taken?</title>
	<meta
		name="description"
		content="Drop a photo of a mountain skyline and Viewfinder finds where it was taken, which way it faced, and the ridge it shows."
	/>
</svelte:head>

{#snippet photo()}
	{#if traced || preview}
		<PhotoFrame
			src={imageUrl || preview}
			width={traced?.width ?? 1}
			height={traced?.height ?? 1}
			alt="Your photo with its skyline drawn over it"
			skyline={phase === "uploading" ? [] : skyline}
			horizon={phase === "done" && result ? result.outline : []}
			box={shownBox}
			boxMode={editable || liveBox ? "edit" : "none"}
			hidden={previewBroken}
			onerror={() => {
				// A HEIC preview the browser cannot show; the locator's JPEG replaces it.
				previewBroken = !imageUrl;
			}}
			onload={() => {
				previewBroken = false;
			}}
		>
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
			{/if}
			{#if result && (phase === "retracing" || phase === "searching")}
				<div class="status">
					<span class="spin" aria-hidden="true">◐</span>
					{phase === "retracing" ? "Retracing…" : `Searching again · ${elapsed.toFixed(1)} s`}
				</div>
			{/if}
		</PhotoFrame>
	{/if}
{/snippet}

{#if phase === "idle"}
	<section class="hero">
		<h1>Where was this<br />photo taken?</h1>
		<p class="sub">Drop a photo of a mountain skyline.</p>
	</section>

	<label
		class={["drop", { dragging }]}
		ondragover={(e) => {
			e.preventDefault();
			dragging = true;
		}}
		ondragleave={() => (dragging = false)}
		ondrop={drop}
	>
		<input type="file" accept="image/*,.heic,.heif" onchange={pick} />
		<span class="pill big">Choose a photo</span>
		<span class="small">or drop it here · JPEG, PNG, HEIC</span>
	</label>

	<button type="button" class="sample" onclick={sample}>
		<img src="/sandy.jpg" alt="" width="84" height="63" />
		<span><b>Try a sample</b><br /><span class="small">Sandy, Utah</span></span>
	</button>

	{#if server === "down"}
		<p class="down">
			{#if dev}
				The locator isn’t running. Start it with <code>pnpm sight</code>, then
			{:else}
				The locator isn’t answering yet. It may still be waking up;
			{/if}
			<button type="button" class="textlink" onclick={checkServer}>check again</button>.
		</p>
	{/if}

	<section class="coverage">
		<div class="cov-text">
			<div class="eyebrow">Works in</div>
			<div class="cov-title">Wasatch Front, Utah</div>
			<div class="cov-note">
				40–41° N, 111–112° W<br />Provo to Salt Lake City.<br />Needs a visible skyline.
			</div>
			<a href={resolve("/examples")}>See example photos</a>
		</div>
		<CoverageMap />
	</section>
{:else if phase === "failed" && problem}
	<section class="problem">
		<div class="eyebrow">{problem.code}</div>
		<h1>{problem.title}</h1>
		<p>{problem.body}</p>
		<div class="row">
			{#if problem.action === "retry"}
				<button type="button" class="pill big" onclick={retry}>Try again</button>
			{:else}
				<button type="button" class="pill big" onclick={reset}>Choose another photo</button>
			{/if}
			<a href={resolve("/how")}>How it works</a>
		</div>
	</section>
{:else if result}
	<ResultView
		{result}
		height={traced?.height ?? 1}
		{photo}
		editable
		busy={busy}
		{shareUrl}
		{shareNote}
	>
		{#snippet tools()}
			<div class="tools">
				{#if problem}<p class="inline-problem">{problem.title} {problem.body}</p>{/if}
				<div class="lens">
					{#if editingFov}
						<label>
							Lens width
							<input
								type="text"
								inputmode="decimal"
								bind:value={fovText}
								placeholder="any"
								size="5"
							/>°
						</label>
						<button type="button" class="pill dark" onclick={search} disabled={busy}
							>Search again</button
						>
					{:else}
						<span
							>Lens {lensKnown ? `${fovText}°${traced?.fov ? " from the photo" : ""}` : "unknown, tried 40–115°"}</span
						>
						<button type="button" class="textlink" onclick={() => (editingFov = true)}
							>change</button
						>
					{/if}
				</div>
				{#if box}
					<button type="button" class="textlink" onclick={() => retrace(null)} disabled={busy}
						>Use the whole photo again</button
					>
				{/if}
			</div>
		{/snippet}
	</ResultView>
{:else if phase === "confirm" && problem}
	<section class="problem compact">
		<div class="eyebrow">{problem.code}</div>
		<h1>{problem.title}</h1>
		<p>{problem.body}</p>
	</section>
	{@render photo()}
	<div class="row after">
		<button type="button" class="pill" onclick={search}>Search again</button>
		{#if box}
			<button type="button" class="textlink" onclick={() => retrace(null)}>Use the whole photo</button>
		{/if}
		<button type="button" class="textlink" onclick={reset}>Choose another photo</button>
	</div>
{:else}
	<section class="working">
		<div class="eyebrow">Working</div>
		<h1>{phase === "searching" ? "Searching" : box ? "Tracing inside your box" : "Tracing the skyline"}</h1>
	</section>
	{@render photo()}
	<div class="progress">
		<ol class="steps">
			{#each steps as s (s.name)}
				<li class={s.state}>
					<span class="dot" aria-hidden="true"
						>{s.state === "done" ? "●" : s.state === "now" ? "◐" : "○"}</span
					>{s.name}
				</li>
			{/each}
		</ol>
		<div class="clockbox">
			<div class="clock">{elapsed.toFixed(1)} s</div>
			<div class="bar"><div style:width="{progress}%"></div></div>
			<div class="small">
				{#if phase === "searching"}
					{lensKnown
						? `Lens from photo: ${fovText}°`
						: "Lens unknown · trying every lens from 40° to 115°"}
				{:else}
					The first photo after a quiet spell waits about 5 s for the server to wake.
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	h1 {
		margin: 0;
		font: 400 clamp(38px, 7vw, 64px) / 1.02 var(--serif);
		letter-spacing: -0.025em;
	}

	.small {
		color: var(--faint);
		font-size: 14px;
	}

	.row {
		display: flex;
		flex-wrap: wrap;
		gap: 12px 22px;
		align-items: center;
	}

	/* Start */
	.hero {
		display: grid;
		gap: 28px;
		padding: 56px 0 32px;
	}

	.hero h1 {
		font-size: clamp(44px, 9vw, 84px);
		line-height: 1;
	}

	.sub {
		margin: 0;
		color: var(--muted);
		font-size: 18px;
	}

	.drop {
		position: relative;
		display: grid;
		justify-items: center;
		gap: 12px;
		padding: 44px 24px;
		border: 1.5px dashed var(--dash);
		border-radius: 16px;
		text-align: center;
		cursor: pointer;
		transition:
			border-color 0.15s,
			background 0.15s;
	}

	.drop:hover,
	.drop.dragging {
		border-color: var(--accent);
		background: var(--soft);
	}

	.drop input {
		position: absolute;
		width: 1px;
		height: 1px;
		opacity: 0;
	}

	.sample {
		display: flex;
		gap: 14px;
		align-items: center;
		margin-top: 14px;
		padding: 0;
		border: 0;
		background: none;
		color: var(--ink);
		text-align: left;
	}

	.sample img {
		width: 84px;
		height: 63px;
		flex: none;
		border-radius: 10px;
		object-fit: cover;
	}

	.sample:hover b {
		color: var(--accent);
	}

	.down {
		margin: 18px 0 0;
		color: var(--poor);
		font-size: 14px;
	}

	code {
		font: 500 13px var(--mono);
	}

	.coverage {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 300px), 1fr));
		gap: 24px;
		align-items: center;
		margin-top: 56px;
		padding-top: 24px;
		border-top: 1px solid var(--rule);
	}

	.cov-text {
		display: grid;
		gap: 6px;
		justify-items: start;
	}

	.cov-title {
		font: 400 28px / 1.15 var(--serif);
	}

	.cov-note {
		color: var(--muted);
		font-size: 14px;
	}

	.cov-text a {
		margin-top: 6px;
		font-size: 14px;
		font-weight: 500;
	}

	/* Working */
	.working {
		display: grid;
		gap: 8px;
		padding: 40px 0 20px;
	}

	.working h1 {
		font-size: clamp(34px, 6vw, 46px);
		line-height: 1.05;
	}

	.progress {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 300px), 1fr));
		gap: 24px;
		padding-top: 24px;
	}

	.steps {
		display: grid;
		gap: 8px;
		align-content: start;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.steps li {
		display: flex;
		gap: 12px;
	}

	.dot {
		width: 16px;
	}

	.steps .now {
		font-weight: 700;
	}

	.steps .next {
		color: #aaa;
	}

	.clockbox {
		display: grid;
		gap: 10px;
		align-content: start;
	}

	.clock {
		font: 400 44px / 1 var(--serif);
		font-variant-numeric: tabular-nums;
	}

	.bar {
		height: 4px;
		border-radius: 2px;
		background: #eee3da;
	}

	.bar div {
		height: 4px;
		border-radius: 2px;
		background: var(--accent);
		transition: width 0.1s linear;
	}

	/* Problems */
	.problem {
		display: grid;
		gap: 20px;
		justify-items: start;
		padding: 64px 0 24px;
	}

	.problem.compact {
		gap: 12px;
		padding: 40px 0 24px;
	}

	.problem p {
		margin: 0;
		max-width: 44ch;
		color: var(--ink-2);
		font-size: 18px;
	}

	.after {
		padding-top: 20px;
	}

	/* Result extras */
	.tools {
		display: grid;
		gap: 10px;
		justify-items: start;
		font-size: 14px;
		color: var(--muted);
	}

	.lens {
		display: flex;
		flex-wrap: wrap;
		gap: 10px 14px;
		align-items: center;
	}

	.lens input {
		width: 5em;
		margin: 0 4px 0 8px;
		padding: 6px 10px;
		border: 1px solid var(--rule-2);
		border-radius: 8px;
		font: inherit;
	}

	.inline-problem {
		margin: 0;
		color: var(--poor);
	}

	/* Over the photo */
	.canvas {
		position: absolute;
		inset: -14px;
		padding: 0;
		border: 0;
		background: transparent;
		touch-action: none;
	}

	.canvas:focus-visible {
		outline-offset: -14px;
	}

	.status {
		position: absolute;
		left: 12px;
		top: 12px;
		display: flex;
		gap: 6px;
		align-items: center;
		padding: 8px 14px;
		border-radius: 99px;
		background: var(--ink);
		color: #fff;
		font-size: 13px;
		line-height: 1.2;
		font-variant-numeric: tabular-nums;
		pointer-events: none;
	}

	.spin {
		display: inline-block;
		animation: spin 1s linear infinite;
	}

	@media (prefers-reduced-motion: reduce) {
		.spin,
		.bar div,
		.drop {
			animation: none;
			transition: none;
		}
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
