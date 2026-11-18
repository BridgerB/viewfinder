// Saves a located photo so it can be shared at /p/{id}: the upright photo (fetched from the
// locator by its upload id) goes to R2, the result to D1. Anyone can call this, so the body is
// checked for shape and size rather than trusted.

import { env } from "cloudflare:workers";
import { error, json } from "@sveltejs/kit";
import { getDb } from "#lib/server/db/index.ts";
import { share } from "#lib/server/db/schema.ts";
import { sight } from "#lib/server/sight.ts";
import type { RequestHandler } from "./$types";

/** The saved result JSON may be at most this long; a real one is ~70 KB. */
const MAX_RESULT = 1_000_000;

type Body = {
	upload: number;
	width: number;
	height: number;
	skyline: [number, number][];
	box: [number, number, number, number] | null;
	result: {
		best: unknown;
		top: unknown[];
		surface: unknown[];
		outline: unknown[];
		ridge: unknown[];
	};
};

const isNumber = (v: unknown): v is number =>
	typeof v === "number" && Number.isFinite(v);
const isPair = (v: unknown) =>
	Array.isArray(v) && v.length === 2 && v.every(isNumber);

const valid = (b: Partial<Body>): b is Body =>
	isNumber(b.upload) &&
	isNumber(b.width) &&
	isNumber(b.height) &&
	Array.isArray(b.skyline) &&
	b.skyline.every(isPair) &&
	(b.box === null ||
		(Array.isArray(b.box) && b.box.length === 4 && b.box.every(isNumber))) &&
	typeof b.result === "object" &&
	b.result !== null &&
	typeof b.result.best === "object" &&
	["top", "surface", "outline", "ridge"].every((k) =>
		Array.isArray((b.result as Record<string, unknown>)[k]),
	);

export const POST: RequestHandler = async ({ request }) => {
	const body = (await request.json().catch(() => ({}))) as Partial<Body>;
	if (!valid(body)) error(400, "not a located photo");
	const result = JSON.stringify(body.result);
	if (result.length > MAX_RESULT) error(413, "result too large");

	const photo = await sight(`/image/${body.upload}`);
	if (!photo.ok) error(409, "that upload has expired; upload the photo again");

	const id = crypto.randomUUID();
	await env.PHOTOS.put(`photos/${id}.jpg`, await photo.arrayBuffer(), {
		httpMetadata: { contentType: "image/jpeg" },
	});
	await getDb(env.DB)
		.insert(share)
		.values({
			id,
			createdAt: new Date(),
			width: Math.round(body.width),
			height: Math.round(body.height),
			skyline: JSON.stringify(body.skyline),
			box: body.box ? JSON.stringify(body.box) : null,
			result,
		});
	return json({ id });
};
