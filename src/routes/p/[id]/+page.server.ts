// A shared result: the row saved by POST /api/shares.

import { env } from "cloudflare:workers";
import { error } from "@sveltejs/kit";
import { eq } from "drizzle-orm";
import type { Located, Point } from "#lib/locate.ts";
import { isShareId } from "#lib/locate.ts";
import { getDb } from "#lib/server/db/index.ts";
import { share } from "#lib/server/db/schema.ts";
import type { PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({ params }) => {
	if (!isShareId(params.id)) error(404, "No photo here");
	const [row] = await getDb(env.DB)
		.select()
		.from(share)
		.where(eq(share.id, params.id))
		.limit(1);
	if (!row) error(404, "No photo here");
	return {
		id: row.id,
		createdAt: row.createdAt.toISOString(),
		width: row.width,
		height: row.height,
		skyline: JSON.parse(row.skyline) as Point[],
		box: row.box
			? (JSON.parse(row.box) as [number, number, number, number])
			: null,
		result: JSON.parse(row.result) as Located,
	};
};
