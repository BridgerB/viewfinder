// The brain's checker. Holds the four invariants at zero:
//   broken links, ambiguous links, orphans, unresolved sourceRefs.
// Also refuses a `verified` note with no sourceRefs or no verifiedAgainst pin.
//
//   node brain/check.ts                 check everything
//   node brain/check.ts affected        notes citing files changed in the working tree
//   node brain/check.ts affected a b    notes citing the given paths
//
// A sourceRef is `path#anchor`, path relative to the repository root. The anchor is the text
// the source says (a declaration like `pub fn view_into(`, or a quoted line), matched after
// normalising both sides the same way, and it must match exactly one place in the file.

import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { basename, dirname, join, relative } from "node:path";

const brain = dirname(new URL(import.meta.url).pathname);
const repo = dirname(brain);

const notes: string[] = [];
const walk = (dir: string) => {
	for (const name of readdirSync(dir)) {
		const p = join(dir, name);
		if (statSync(p).isDirectory()) walk(p);
		else if (name.endsWith(".md")) notes.push(p);
	}
};
walk(brain);

const rel = (p: string) => relative(brain, p).replace(/\.md$/, "");

type Front = Record<string, string | string[]>;
const frontmatter = (text: string): Front => {
	const m = text.match(/^---\n([\s\S]*?)\n---/);
	const out: Front = {};
	if (!m) return out;
	let key = "";
	for (const line of m[1].split("\n")) {
		const item = line.match(/^\s+-\s+(.*)$/);
		if (item && key) {
			const list = Array.isArray(out[key]) ? (out[key] as string[]) : [];
			list.push(item[1].replace(/^"(.*)"$/, "$1"));
			out[key] = list;
			continue;
		}
		const kv = line.match(/^(\w+):\s*(.*)$/);
		if (kv) {
			key = kv[1];
			out[key] = kv[2];
		}
	}
	return out;
};

// What a line says, not how it is written.
const plain = (s: string) =>
	s
		.replace(/^\s*(?:[-*+]|\d+\.)\s+/, "")
		.replace(/[*_`#]/g, "")
		.replace(/\s+/g, " ")
		.trim();

const flatten = (text: string) => {
	let flat = "";
	const lineOf: number[] = [];
	text.split("\n").forEach((raw, i) => {
		const s = plain(raw);
		if (!s) return;
		const piece = ` ${s}`;
		flat += piece;
		for (let k = 0; k < piece.length; k++) lineOf.push(i + 1);
	});
	return { flat, lineOf };
};

const occurrences = (hay: string, needle: string) => {
	const at: number[] = [];
	if (!needle) return at;
	let i = hay.indexOf(needle);
	while (i >= 0) {
		at.push(i);
		i = hay.indexOf(needle, i + needle.length);
	}
	return at;
};

const refsOf = (front: Front) =>
	Array.isArray(front.sourceRefs) ? front.sourceRefs : [];

if (process.argv[2] === "affected") {
	const paths =
		process.argv.length > 3
			? process.argv.slice(3)
			: execFileSync("git", ["-C", repo, "status", "--porcelain"], {
					encoding: "utf8",
				})
					.split("\n")
					.filter(Boolean)
					.map((l) => l.slice(3).trim());
	for (const n of notes) {
		const cited = refsOf(frontmatter(readFileSync(n, "utf8"))).map(
			(r) => r.split("#")[0],
		);
		const hit = paths.filter((p) =>
			cited.some((c) => c === p || c.startsWith(`${p.replace(/\/$/, "")}/`)),
		);
		if (hit.length) console.log(`${rel(n)}  <-  ${hit.join(", ")}`);
	}
	process.exit(0);
}

const byPath = new Map<string, string>();
const byStem = new Map<string, string[]>();
for (const n of notes) {
	const r = rel(n);
	byPath.set(r, n);
	const stem = basename(r);
	byStem.set(stem, [...(byStem.get(stem) ?? []), r]);
}

const broken: string[] = [];
const ambiguous: string[] = [];
const unresolved: string[] = [];
const metadata: string[] = [];
const inbound = new Map<string, number>([...byPath.keys()].map((k) => [k, 0]));

for (const n of notes) {
	const text = readFileSync(n, "utf8");
	const me = rel(n);
	const body = text.replace(/```[\s\S]*?```/g, "").replace(/`[^`\n]*`/g, "");
	for (const m of body.matchAll(/\[\[([^\]]+)\]\]/g)) {
		const target = m[1]
			.replace(/\\\|/g, "|")
			.split("|")[0]
			.split("#")[0]
			.trim();
		if (!target) continue;
		let resolved = byPath.has(target) ? target : undefined;
		if (!resolved) {
			const cands = byStem.get(basename(target)) ?? [];
			if (cands.length === 1) resolved = cands[0];
			else if (cands.length > 1) {
				ambiguous.push(`${me}: [[${target}]] -> ${cands.join(", ")}`);
				continue;
			} else {
				broken.push(`${me}: [[${target}]]`);
				continue;
			}
		}
		if (resolved !== me)
			inbound.set(resolved, (inbound.get(resolved) ?? 0) + 1);
	}
	const front = frontmatter(text);
	const refs = refsOf(front);
	if (front.status === "verified") {
		if (
			!refs.length &&
			!["index", "moc", "governing"].includes(String(front.type))
		)
			metadata.push(`${me}: verified with no sourceRefs`);
		if (!front.verifiedAgainst)
			metadata.push(`${me}: verified with no verifiedAgainst`);
	}
	for (const ref of refs) {
		const cut = ref.indexOf("#");
		const path = cut < 0 ? ref : ref.slice(0, cut);
		const anchor = cut < 0 ? "" : ref.slice(cut + 1);
		const file = join(repo, path);
		if (!existsSync(file)) {
			unresolved.push(`${me}: ${ref}  (no such file)`);
			continue;
		}
		if (!anchor) {
			unresolved.push(`${me}: ${ref}  (no anchor: a bare path cites nothing)`);
			continue;
		}
		const { flat, lineOf } = flatten(readFileSync(file, "utf8"));
		const hits = occurrences(flat, plain(anchor)).map((i) => lineOf[i]);
		if (hits.length === 0) unresolved.push(`${me}: ${ref}  (matches nothing)`);
		else if (hits.length > 1)
			unresolved.push(
				`${me}: ${ref}  (matches ${hits.length}: lines ${hits.join(", ")})`,
			);
	}
}

const isHub = (r: string) =>
	/(^|\/)(_moc|_index)$/.test(r) ||
	[
		"operating-manual",
		"conventions",
		"rules",
		"glossary",
		"questions",
	].includes(r);
const orphans = [...inbound.entries()]
	.filter(([r, c]) => c === 0 && !isHub(r))
	.map(([r]) => r);

const report = (name: string, list: string[]) => {
	console.log(`${name}: ${list.length}`);
	for (const x of list) console.log(`  ${x}`);
};
console.log(`notes: ${notes.length}`);
report("broken links", broken);
report("ambiguous links", ambiguous);
report("orphans", orphans);
report("unresolved sourceRefs", unresolved);
report("metadata", metadata);
process.exit(
	broken.length +
		ambiguous.length +
		orphans.length +
		unresolved.length +
		metadata.length
		? 1
		: 0,
);
