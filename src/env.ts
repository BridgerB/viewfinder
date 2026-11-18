import { defineEnvVars } from "@sveltejs/kit/env";
import { building } from "$app/env";

/**
 * Required when the app starts, optional while building. These are dynamic (read at runtime
 * from the Worker's secrets, never inlined), so Workers Builds can build without them.
 */
const requiredAtRuntime = (name: string) => (value: string | undefined) => {
	if (!value && !building) throw new Error(`${name} is not set`);
	return value ?? "";
};

export const variables = defineEnvVars({
	ORIGIN: {
		description: "The app origin (base URL), e.g. `http://localhost:4372`.",
		schema: requiredAtRuntime("ORIGIN"),
	},
	BETTER_AUTH_SECRET: {
		description:
			"Secret used to sign tokens. For production use 32 characters generated with high entropy. See [Better Auth installation](https://www.better-auth.com/docs/installation).",
		schema: requiredAtRuntime("BETTER_AUTH_SECRET"),
	},
});
