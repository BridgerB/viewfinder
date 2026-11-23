// The /dev review page exists only under `pnpm dev`: its data comes from the gitignored
// data/shares/ through a Vite dev middleware (vite.config.ts), which a deployed site does not have.
import { error } from "@sveltejs/kit";
import { dev } from "$app/env";

export const ssr = false;
export const prerender = false;

export const load = () => {
	if (!dev) error(404, "Not found");
};
