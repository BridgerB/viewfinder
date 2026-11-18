// The locator behind a Durable Object: one container running `sight serve`, started on the first
// request and stopped after a quiet spell. Requests arrive from the site through a service
// binding, already stripped to the locator's own paths (/health, /trace, /image/ID, /locate).
import { DurableObject } from "cloudflare:workers";

/** The port `sight serve` listens on inside the container (see Dockerfile). */
const PORT = 4373;
/** Stop the container after this long without a request; the next one starts it again. */
const IDLE_MS = 15 * 60 * 1000;
/** 4 vCPU and 12 GiB: a search uses every core, and the grid and fields need ~300 MB. */
const INSTANCE = "standard-4";

export class Sight extends DurableObject<Env> {
	private starting: Promise<Container> | undefined;

	/** The container, started if it is not running, once its server answers /health. */
	private async ready(): Promise<Container> {
		const container = this.ctx.container;
		if (!container) throw new Error("this Durable Object has no container");
		if (!container.running) {
			const image = container.images.base;
			if (!image) throw new Error("the container image 'base' is missing");
			container.start({ image, instance: INSTANCE, enableInternet: false });
		}
		await container.setInactivityTimeout(IDLE_MS);
		const port = container.getTcpPort(PORT);
		// Loading the grid and fields takes well under a second; the start itself takes seconds.
		for (let attempt = 0; attempt < 150; attempt++) {
			try {
				const res = await port.fetch("http://container/health", {
					signal: AbortSignal.timeout(1000),
				});
				if (res.ok) return container;
			} catch {
				// Not listening yet.
			}
			await scheduler.wait(200);
		}
		throw new Error(`the locator did not start on port ${PORT}`);
	}

	override async fetch(request: Request): Promise<Response> {
		this.starting ??= this.ready().finally(() => {
			this.starting = undefined;
		});
		const container = await this.starting;
		const url = new URL(request.url);
		const inside = `http://container${url.pathname}${url.search}`;
		return container.getTcpPort(PORT).fetch(new Request(inside, request));
	}
}

export default {
	fetch(request, env) {
		// One instance holds every upload, so a photo and the searches on it reach the same memory.
		return env.SIGHT.getByName("locator").fetch(request);
	},
} satisfies ExportedHandler<Env>;
