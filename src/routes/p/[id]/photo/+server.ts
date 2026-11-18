// The photo of a shared result, from R2. A share never changes, so it caches for a year.

import { env } from "cloudflare:workers";
import { error } from "@sveltejs/kit";
import { isShareId } from "#lib/locate.ts";
import type { RequestHandler } from "./$types";

export const GET: RequestHandler = async ({ params }) => {
	if (!isShareId(params.id)) error(404, "no such photo");
	const object = await env.PHOTOS.get(`photos/${params.id}.jpg`);
	if (!object) error(404, "no such photo");
	return new Response(object.body, {
		headers: {
			"Content-Type": "image/jpeg",
			"Cache-Control": "public, max-age=31536000, immutable",
		},
	});
};
