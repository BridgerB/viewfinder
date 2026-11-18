// /api/sight/* → the locator, same-origin for the /locate page. Deployed, that is the
// viewfinder-sight Worker's container (service binding SIGHT). In `pnpm dev` it is
// `pnpm sight` running beside the dev server on port 4373.
import { env } from "cloudflare:workers";
import { dev } from "$app/env";
import type { RequestHandler } from "./$types";

/** Headers about this hop's connection, not the request; a proxy must not pass them on. */
const HOP = [
	"connection",
	"expect",
	"host",
	"keep-alive",
	"transfer-encoding",
	"upgrade",
];

const forward: RequestHandler = async ({ params, request, url }) => {
	const path = `/${params.path}${url.search}`;
	const headers = new Headers(request.headers);
	for (const name of HOP) headers.delete(name);
	const method = request.method;
	if (dev) {
		// Node's fetch will not stream a request body without extra options; a photo fits in memory.
		const body = method === "GET" ? undefined : await request.arrayBuffer();
		return fetch(`http://127.0.0.1:4373${path}`, { method, headers, body });
	}
	return env.SIGHT.fetch(`https://sight${path}`, {
		method,
		headers,
		body: request.body,
	});
};

export const GET = forward;
export const POST = forward;
