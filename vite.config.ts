import { createReadStream, existsSync, writeFileSync } from "node:fs";
import { join, normalize } from "node:path";
import { autoPort } from "@bridgerb/port-from-name";
import adapter from "@sveltejs/adapter-cloudflare";
import { enhancedImages } from "@sveltejs/enhanced-img";
import { sveltekit } from "@sveltejs/kit/vite";
import { playwright } from "@vitest/browser-playwright";
import type { Plugin } from "vite";
import { defineConfig } from "vitest/config";

/**
 * In `pnpm dev` only, serves the gitignored data/shares/ (downloaded share rows and photos) at
 * /dev-data/ for the /dev review page, and saves the places marked there (`PUT
 * /dev-data/truth.json`) to data/shares/truth.json. Nothing here is built or deployed.
 */
const devShares = (): Plugin => ({
	name: "dev-shares",
	apply: "serve",
	configureServer(server) {
		const root = join(process.cwd(), "data", "shares");
		server.middlewares.use("/dev-data", (req, res, next) => {
			if (req.method === "PUT" && req.url === "/truth.json") {
				const chunks: Buffer[] = [];
				req.on("data", (c: Buffer) => chunks.push(c));
				req.on("end", () => {
					const body = Buffer.concat(chunks).toString("utf8");
					try {
						JSON.parse(body); // Refuse anything that is not JSON.
						writeFileSync(join(root, "truth.json"), body);
						res.statusCode = 204;
					} catch {
						res.statusCode = 400;
					}
					res.end();
				});
				return;
			}
			const name = normalize(
				decodeURIComponent((req.url ?? "/").split("?")[0]),
			).replace(/^[/\\]+/, "");
			const path = join(root, name);
			if (!path.startsWith(root) || !existsSync(path)) return next();
			res.setHeader(
				"Content-Type",
				path.endsWith(".json") ? "application/json" : "image/jpeg",
			);
			res.setHeader("Cache-Control", "no-store");
			createReadStream(path).pipe(res);
		});
	},
});

/**
 * In a build, the /dev page is an empty component, so the review page, its map picker and
 * anything they import never reach the deployed bundle. Its `+page.ts` answers 404 outside
 * `pnpm dev` either way.
 */
const devPageOnlyInDev = (): Plugin => ({
	name: "dev-page-only-in-dev",
	apply: "build",
	enforce: "pre",
	load(id) {
		if (
			/[/\\]src[/\\]routes[/\\]dev[/\\]\+page\.svelte$/.test(id.split("?")[0])
		)
			return "";
	},
});

export default defineConfig({
	plugins: [
		devPageOnlyInDev(),
		enhancedImages(),
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes("node_modules") ? undefined : true,
			},
			adapter: adapter(),
		}),
		autoPort(),
		devShares(),
	],
	test: {
		expect: { requireAssertions: true },
		projects: [
			{
				extends: "./vite.config.ts",
				test: {
					name: "client",
					browser: {
						enabled: true,
						provider: playwright(),
						instances: [{ browser: "chromium", headless: true }],
					},
					include: ["src/**/*.svelte.{test,spec}.{js,ts}"],
					exclude: ["src/lib/server/**"],
				},
			},

			{
				extends: "./vite.config.ts",
				test: {
					name: "server",
					environment: "node",
					include: ["src/**/*.{test,spec}.{js,ts}"],
					exclude: ["src/**/*.svelte.{test,spec}.{js,ts}"],
				},
			},
		],
	},
});
