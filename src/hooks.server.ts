import type { Handle } from "@sveltejs/kit/hooks";
import { svelteKitHandler } from "better-auth/svelte-kit";
import { createAuth } from "#lib/server/auth.ts";
import { building } from "$app/env";

const handleBetterAuth: Handle = async ({ event, resolve }) => {
	event.locals.auth = createAuth();

	const { auth } = event.locals;
	const session = await auth.api.getSession({ headers: event.request.headers });

	if (session) {
		event.locals.session = session.session;
		event.locals.user = session.user;
	}

	return svelteKitHandler({ event, resolve, auth, building });
};

export const handle: Handle = handleBetterAuth;
