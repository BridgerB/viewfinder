import { integer, sqliteTable, text } from "drizzle-orm/sqlite-core";

export const task = sqliteTable("task", {
	id: text("id")
		.primaryKey()
		.$defaultFn(() => crypto.randomUUID()),
	title: text("title").notNull(),
	priority: integer("priority").notNull().default(1),
});

/**
 * A located photo anyone with its link can open at /p/{id}. The id is a random UUID, so the
 * pages are unlisted but not private. The photo is in R2 at `photos/{id}.jpg`.
 */
export const share = sqliteTable("share", {
	id: text("id").primaryKey(),
	createdAt: integer("created_at", { mode: "timestamp" }).notNull(),
	/** The upright photo's size in pixels; the skyline and box are in these pixels. */
	width: integer("width").notNull(),
	height: integer("height").notNull(),
	/** The skyline the search used, as [x, y] pairs (JSON). */
	skyline: text("skyline").notNull(),
	/** The box it was traced in, [x0, y0, x1, y1] (JSON), or null for the whole photo. */
	box: text("box"),
	/** The locator's full answer, as `Located` in #lib/locate.ts (JSON). */
	result: text("result").notNull(),
});

export * from "./auth.schema";
