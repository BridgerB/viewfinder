// Sends every downloaded share group (data/shares/rows.json: retries of one photo are uploaded one
// after another at the same size) through the local locator, `pnpm sight` behind `pnpm dev`, and
// writes the answers to data/shares/reruns.json for the /dev review page.
//
//   node scripts/rerun-shares.ts [SITE=http://localhost:4372]
import { readFileSync, writeFileSync } from "node:fs";

type Row = { id: string; width: number; height: number };
const site = process.argv[2] ?? "http://localhost:4372";
const rows = (
	JSON.parse(readFileSync("data/shares/rows.json", "utf8")) as {
		results: Row[];
	}[]
)[0].results;

const firsts: Row[] = [];
for (const [i, r] of rows.entries()) {
	const prev = rows[i - 1];
	if (!prev || prev.width !== r.width || prev.height !== r.height)
		firsts.push(r);
}

const out: Record<string, unknown> = {};
for (const [n, r] of firsts.entries()) {
	const started = performance.now();
	const photo = readFileSync(`data/shares/${r.id}.jpg`);
	const traced = await (
		await fetch(`${site}/api/sight/trace`, { method: "POST", body: photo })
	).json();
	const res = await fetch(`${site}/api/sight/locate`, {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify({ id: traced.id, fov: traced.fov }),
	});
	const result = await res.json();
	const seconds = (performance.now() - started) / 1000;
	out[r.id] = res.ok
		? {
				width: traced.width,
				height: traced.height,
				skyline: result.skyline,
				result,
				seconds,
			}
		: { error: result.error ?? res.statusText, seconds };
	const b = result.best;
	console.log(
		`#${n + 1} ${r.id.slice(0, 8)} ${res.ok ? `${b.lat.toFixed(4)}, ${b.lon.toFixed(4)} yaw ${b.yaw} fov ${b.fov} loss ${b.loss.toFixed(3)}` : result.error} ${seconds.toFixed(1)} s`,
	);
	writeFileSync("data/shares/reruns.json", JSON.stringify(out));
}
