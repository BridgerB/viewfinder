<script lang="ts">
import { onMount } from "svelte";
import {
	API,
	type Box,
	type Located,
	type Point,
	shapeOf,
	spreadKm,
	type Traced,
	verdict,
} from "#lib/locate.ts";
import PhotoFrame from "#lib/PhotoFrame.svelte";
import { compass, placeName } from "#lib/places.ts";
import TruthMap, { offset, type Truth } from "./TruthMap.svelte";

/** One saved share, as downloaded from D1 into data/shares/rows.json. */
type Row = {
	id: string;
	created_at: number;
	width: number;
	height: number;
	skyline: string;
	box: string | null;
	result: string;
};
type Attempt = {
	id: string;
	at: Date;
	width: number;
	height: number;
	skyline: Point[];
	box: Box | null;
	result: Located;
};
/** A rerun on the current local locator. */
type Rerun = {
	state: "running" | "done" | "failed";
	width?: number;
	height?: number;
	skyline?: Point[];
	result?: Located;
	error?: string;
	seconds?: number;
};

let groups = $state<{ n: number; attempts: Attempt[] }[]>([]);
let loadError = $state("");
let reruns = $state<Record<number, Rerun>>({});
let notes = $state<Record<number, string>>({});
let copied = $state(false);
/** Where each photo was really taken, by photo id of the group's first attempt. */
let truths = $state<Record<string, Truth>>({});
let truthSaved = $state("");

const NOTES_KEY = "viewfinder-dev-notes";

onMount(async () => {
	try {
		notes = JSON.parse(localStorage.getItem(NOTES_KEY) ?? "{}");
	} catch {
		notes = {};
	}
	try {
		const res = await fetch("/dev-data/truth.json");
		if (res.ok) truths = await res.json();
	} catch {
		truths = {};
	}
	try {
		const raw = (await (await fetch("/dev-data/rows.json")).json()) as {
			results: Row[];
		}[];
		const attempts: Attempt[] = raw[0].results.map((r) => ({
			id: r.id,
			at: new Date(r.created_at * 1000),
			width: r.width,
			height: r.height,
			skyline: JSON.parse(r.skyline),
			box: r.box ? JSON.parse(r.box) : null,
			result: JSON.parse(r.result),
		}));
		// Retries of one photo (same size, uploaded one after another) form one group.
		const out: { n: number; attempts: Attempt[] }[] = [];
		for (const a of attempts) {
			const last = out.at(-1);
			const prev = last?.attempts.at(-1);
			if (last && prev && prev.width === a.width && prev.height === a.height)
				last.attempts.push(a);
			else out.push({ n: out.length + 1, attempts: [a] });
		}
		groups = out;
		// Reruns saved by scripts/rerun-shares.ts, keyed by each group's first photo.
		const saved = await fetch("/dev-data/reruns.json");
		if (saved.ok) {
			const byId = (await saved.json()) as Record<string, Rerun>;
			for (const g of out) {
				const r = byId[g.attempts[0].id];
				if (r) reruns[g.n] = { ...r, state: r.error ? "failed" : "done" };
			}
		}
	} catch (e) {
		loadError = `Couldn't read /dev-data/rows.json (${e instanceof Error ? e.message : e}). Download the shares into data/shares/ first.`;
	}
});

const saveNote = (n: number, text: string) => {
	notes[n] = text;
	try {
		localStorage.setItem(NOTES_KEY, JSON.stringify(notes));
	} catch {
		// Private mode: notes stay on the page only.
	}
};

const setTruth = async (id: string, t: Truth) => {
	truths[id] = {
		lat: Number(t.lat.toFixed(5)),
		lon: Number(t.lon.toFixed(5)),
		yaw: Math.round(t.yaw),
	};
	truthSaved = "Saving…";
	try {
		const res = await fetch("/dev-data/truth.json", {
			method: "PUT",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify(truths, null, 2),
		});
		truthSaved = res.ok
			? "Saved to data/shares/truth.json"
			: `Not saved (${res.status})`;
	} catch (e) {
		truthSaved = `Not saved (${e instanceof Error ? e.message : e})`;
	}
};

/** How far an answer is from the marked place, and how far its heading turns from the marked one. */
const miss = (r: Located, t: Truth) => {
	const km = offset(t, r.best).km;
	const turn = Math.abs(((r.best.yaw - t.yaw + 540) % 360) - 180);
	return `${km.toFixed(2)} km off, heading ${turn.toFixed(0)}° off`;
};

const judge = (result: Located, height: number) =>
	verdict(result.best, spreadKm(result.top), shapeOf(result.skyline, height));

const rerun = async (n: number, photoId: string) => {
	reruns[n] = { state: "running" };
	const started = performance.now();
	try {
		const blob = await (await fetch(`/dev-data/${photoId}.jpg`)).blob();
		const traced = (await (
			await fetch(`${API}/trace`, { method: "POST", body: blob })
		).json()) as Traced & { error?: string };
		if (traced.error) throw new Error(traced.error);
		const res = await fetch(`${API}/locate`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({ id: traced.id, fov: traced.fov }),
		});
		const result = (await res.json()) as Located & { error?: string };
		if (!res.ok) throw new Error(result.error ?? res.statusText);
		reruns[n] = {
			state: "done",
			width: traced.width,
			height: traced.height,
			skyline: result.skyline,
			result,
			seconds: (performance.now() - started) / 1000,
		};
	} catch (e) {
		reruns[n] = {
			state: "failed",
			error: e instanceof Error ? e.message : String(e),
		};
	}
};

const rerunAll = async () => {
	for (const g of groups) await rerun(g.n, g.attempts[0].id);
};

const copyNotes = async () => {
	const text = groups
		.filter((g) => notes[g.n]?.trim() || truths[g.attempts[0].id])
		.map((g) => {
			const t = truths[g.attempts[0].id];
			const at = t ? ` [true: ${t.lat}, ${t.lon} facing ${t.yaw}°]` : "";
			return `#${g.n} (${g.attempts.map((a) => a.id.slice(0, 8)).join(", ")})${at}: ${notes[g.n]?.trim() ?? ""}`;
		})
		.join("\n");
	try {
		await navigator.clipboard.writeText(text);
		copied = true;
		setTimeout(() => {
			copied = false;
		}, 1600);
	} catch {
		// The notes are still on the page.
	}
};

const summary = (r: Located) =>
	`${r.best.lat.toFixed(5)}, ${r.best.lon.toFixed(5)} · facing ${r.best.yaw}° ${compass(r.best.yaw)} · lens ${r.best.fov}° · tilt ${r.best.pitch.toFixed(1)}° · loss ${r.best.loss.toFixed(3)}`;
</script>

<svelte:head>
	<title>Review · Viewfinder dev</title>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<section class="intro">
	<div class="eyebrow">Dev only</div>
	<h1>Review shared photos</h1>
	<p>
		Every photo uploaded to the live site, grouped with its retries. Write what is wrong under each one
		(the yellow line on clouds, buildings, wrong ridge, the cyan line off by a little), then copy the
		notes and paste them into the chat. “Rerun” sends the photo through the local locator, so changes
		to the tracer show up here.
	</p>
	<div class="row">
		<button type="button" class="pill" onclick={rerunAll}>Rerun all locally</button>
		<button type="button" class="pill dark" onclick={copyNotes}
			>{copied ? "Copied ✓" : "Copy all notes"}</button
		>
	</div>
	{#if loadError}<p class="error">{loadError}</p>{/if}
</section>

{#each groups as g (g.n)}
	{@const first = g.attempts[0]}
	{@const truth = truths[first.id]}
	{@const last = g.attempts.at(-1) ?? first}
	{@const local = reruns[g.n]?.result}
	<article class="group">
		<header>
			<h2>#{g.n}</h2>
			<span class="dim">{first.width}×{first.height} · {g.attempts.length} attempt{g.attempts.length > 1 ? "s" : ""}</span>
		</header>

		<div class="attempts">
			{#each g.attempts as a, i (a.id)}
				{@const v = judge(a.result, a.height)}
				<figure>
					<PhotoFrame
						src="/dev-data/{a.id}.jpg"
						width={a.width}
						height={a.height}
						alt="Attempt {i + 1}"
						skyline={a.skyline}
						horizon={a.result.outline}
						box={a.box}
						boxMode={a.box ? "view" : "none"}
					/>
					<figcaption>
						<span class={["level", v.level]}>{v.level}</span>
						<b>Attempt {i + 1}{a.box ? " (box)" : ""}</b> · {placeName([a.result.best.lat, a.result.best.lon])}
						<br /><span class="dim">{summary(a.result)}</span>
						<br /><a href="https://www.google.com/maps?q={a.result.best.lat},{a.result.best.lon}" target="_blank" rel="noopener">map</a>
						· <a href="https://viewfinder.bridgerb.com/p/{a.id}" target="_blank" rel="noopener">live share</a>
						· <code>{a.id.slice(0, 8)}</code>
					</figcaption>
				</figure>
			{/each}

			{#if reruns[g.n]}
				{@const r = reruns[g.n]}
				<figure class="rerun">
					{#if r.state === "running"}
						<div class="placeholder">Running on the local locator…</div>
					{:else if r.state === "failed"}
						<div class="placeholder error">{r.error}</div>
					{:else if r.result && r.width && r.height}
						{@const v = judge(r.result, r.height)}
						<PhotoFrame
							src="/dev-data/{first.id}.jpg"
							width={r.width}
							height={r.height}
							alt="Local rerun"
							skyline={r.skyline}
							horizon={r.result.outline}
						/>
						<figcaption>
							<span class={["level", v.level]}>{v.level}</span>
							<b>Local rerun</b> · {placeName([r.result.best.lat, r.result.best.lon])} · {r.seconds?.toFixed(1)} s
							<br /><span class="dim">{summary(r.result)}</span>
							<br /><a href="https://www.google.com/maps?q={r.result.best.lat},{r.result.best.lon}" target="_blank" rel="noopener">map</a>
						</figcaption>
					{/if}
				</figure>
			{/if}
		</div>

		<div class="truth">
			<TruthMap
				answer={last.result.best}
				rerun={local?.best}
				{truth}
				fov={last.result.best.fov}
				onchange={(t) => setTruth(first.id, t)}
			/>
			<p class="dim">
				{#if truth}
					<b>Marked:</b> {truth.lat}, {truth.lon} facing {truth.yaw}° {compass(truth.yaw)}
					· live answer {miss(last.result, truth)}
					{#if local}· local rerun {miss(local, truth)}{/if}
					{#if truthSaved}<br />{truthSaved}{/if}
				{:else}
					Click the map where the photo was taken, then drag the small handle to the way the
					camera faced. Orange: the live answer; blue dashed: the local rerun.
				{/if}
			</p>
		</div>

		<div class="tools">
			<button type="button" class="textlink" onclick={() => rerun(g.n, first.id)}
				>Rerun locally</button
			>
			<label>
				<span class="dim">What’s wrong with #{g.n}? Where was it really taken?</span>
				<textarea
					rows="2"
					value={notes[g.n] ?? ""}
					oninput={(e) => saveNote(g.n, e.currentTarget.value)}
				></textarea>
			</label>
		</div>
	</article>
{/each}

<style>
	.intro {
		display: grid;
		gap: 12px;
		padding: 40px 0 8px;
	}

	h1 {
		margin: 0;
		font: 400 clamp(36px, 6vw, 52px) / 1.05 var(--serif);
	}

	.intro p {
		margin: 0;
		max-width: 64ch;
		color: var(--ink-2);
	}

	.row {
		display: flex;
		flex-wrap: wrap;
		gap: 10px;
	}

	.group {
		display: grid;
		gap: 14px;
		margin-top: 36px;
		padding-top: 20px;
		border-top: 1px solid var(--rule);
	}

	header {
		display: flex;
		gap: 12px;
		align-items: baseline;
	}

	h2 {
		margin: 0;
		font: 400 32px var(--serif);
	}

	.attempts {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 380px), 1fr));
		gap: 18px;
	}

	figure {
		display: grid;
		gap: 8px;
		align-content: start;
		margin: 0;
	}

	.rerun {
		padding: 10px;
		border: 2px dashed var(--accent);
		border-radius: 16px;
	}

	figcaption {
		font-size: 13px;
		line-height: 1.5;
	}

	.dim {
		color: var(--muted);
		font-size: 13px;
	}

	.level {
		display: inline-block;
		margin-right: 6px;
		padding: 1px 8px;
		border-radius: 99px;
		color: #fff;
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
	}

	.level.good {
		background: var(--good);
	}

	.level.unsure {
		background: var(--unsure);
	}

	.level.poor {
		background: var(--poor);
	}

	code {
		font: 500 12px var(--mono);
	}

	.placeholder {
		display: grid;
		place-items: center;
		min-height: 160px;
		border-radius: 14px;
		background: var(--soft);
		color: var(--muted);
		font-size: 14px;
		text-align: center;
		padding: 12px;
	}

	.error {
		color: var(--poor);
	}

	.truth {
		display: grid;
		gap: 6px;
		max-width: 720px;
	}

	.truth p {
		margin: 0;
	}

	.tools {
		display: grid;
		gap: 8px;
		justify-items: start;
	}

	label {
		display: grid;
		gap: 4px;
		width: 100%;
	}

	textarea {
		width: 100%;
		padding: 10px 12px;
		border: 1px solid var(--rule-2);
		border-radius: 10px;
		font: inherit;
		font-size: 14px;
		resize: vertical;
	}
</style>
