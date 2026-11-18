// /api/sight/* → the locator, same-origin for the /locate page (see #lib/server/sight.ts).
import { sight } from "#lib/server/sight.ts";
import type { RequestHandler } from "./$types";

const forward: RequestHandler = ({ params, request, url }) =>
	sight(`/${params.path}${url.search}`, request);

export const GET = forward;
export const POST = forward;
