// Requests to the locator. Deployed, that is the viewfinder-sight Worker's container through the
// SIGHT service binding; in `pnpm dev` it is `pnpm sight` running beside the dev server.
import { env } from "cloudflare:workers";
import { dev } from "$app/env";

/** Headers about one hop's connection, not the request; a proxy must not pass them on. */
const HOP = [
	"connection",
	"expect",
	"host",
	"keep-alive",
	"transfer-encoding",
	"upgrade",
];

/** Sends `path` (with any query) to the locator, with `request`'s method, headers and body. */
export const sight = async (path: string, request?: Request) => {
	const method = request?.method ?? "GET";
	const headers = new Headers(request?.headers);
	for (const name of HOP) headers.delete(name);
	if (dev) {
		// Node's fetch will not stream a request body without extra options; a photo fits in memory.
		const body =
			request && method !== "GET" ? await request.arrayBuffer() : undefined;
		return fetch(`http://127.0.0.1:4373${path}`, { method, headers, body });
	}
	return env.SIGHT.fetch(`https://sight${path}`, {
		method,
		headers,
		body: request?.body,
	});
};
