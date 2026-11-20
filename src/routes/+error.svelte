<script lang="ts">
import { resolve } from "$app/paths";
import { page } from "$app/state";

const gone = $derived(page.status === 404);
const shared = $derived(page.url.pathname.startsWith("/p/"));
</script>

<svelte:head>
	<title>{gone ? "Not found" : "Something went wrong"} · Viewfinder</title>
</svelte:head>

<section class="problem">
	<div class="eyebrow">{shared && gone ? "Link not found" : gone ? "Not found" : `Error ${page.status}`}</div>
	<h1>{shared && gone ? "This link is gone." : gone ? "Nothing here." : "Something went wrong."}</h1>
	<p>
		{#if shared && gone}
			It may have been removed, or the address is incomplete.
		{:else if gone}
			The address may be mistyped, or the page has moved.
		{:else}
			{page.error?.message ?? "Try again in a moment."}
		{/if}
	</p>
	<div class="row">
		<a class="pill big" href={resolve("/")}>Try your own photo</a>
		<a href={resolve("/how")}>How it works</a>
	</div>
</section>

<style>
	.problem {
		display: grid;
		gap: 20px;
		justify-items: start;
		padding: 64px 0 24px;
	}

	h1 {
		margin: 0;
		font: 400 clamp(38px, 7vw, 64px) / 1.02 var(--serif);
		letter-spacing: -0.025em;
	}

	p {
		margin: 0;
		max-width: 44ch;
		color: var(--ink-2);
		font-size: 18px;
	}

	.row {
		display: flex;
		flex-wrap: wrap;
		gap: 12px 22px;
		align-items: center;
	}
</style>
