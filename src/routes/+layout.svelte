<script lang="ts">
import "../app.css";
import favicon from "#lib/assets/favicon.svg";
import { resolve } from "$app/paths";
import { page } from "$app/state";
import type { LayoutProps } from "./$types";

let { children }: LayoutProps = $props();

const here = $derived(page.url.pathname);
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<link rel="preconnect" href="https://fonts.googleapis.com" />
	<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
	<link
		rel="stylesheet"
		href="https://fonts.googleapis.com/css2?family=Newsreader:ital,wght@0,400;0,500;1,400&family=DM+Sans:wght@400;500;700&family=JetBrains+Mono:wght@500&display=swap"
	/>
</svelte:head>

<div class="site">
	<header>
		<a class="wordmark" href={resolve("/")}>Viewfinder</a>
		<nav>
			<a href={resolve("/examples")} aria-current={here === "/examples" ? "page" : undefined}
				>Examples</a
			>
			<a href={resolve("/how")} aria-current={here === "/how" ? "page" : undefined}>How it works</a>
			<a class="new" href={resolve("/")} data-new-photo>New photo</a>
		</nav>
	</header>
	<main>
		{@render children()}
	</main>
</div>

<style>
	.site {
		max-width: 880px;
		margin: 0 auto;
		padding: 0 max(24px, env(safe-area-inset-right, 0px)) 56px max(24px, env(safe-area-inset-left, 0px));
	}

	header {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		align-items: center;
		gap: 12px;
		padding: 26px 0 8px;
	}

	.wordmark {
		border: 0;
		color: var(--ink);
		font: 500 24px var(--serif);
	}

	nav {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 22px;
		font-size: 14px;
		font-weight: 500;
	}

	nav a {
		border: 0;
		padding: 8px 0;
		color: var(--ink);
	}

	nav a[aria-current="page"] {
		border-bottom: 2px solid var(--ink);
	}

	nav a:hover {
		color: var(--accent);
	}

	nav .new {
		color: var(--accent);
		font-weight: 700;
	}

	@media (max-width: 420px) {
		.site {
			padding-left: max(16px, env(safe-area-inset-left, 0px));
			padding-right: max(16px, env(safe-area-inset-right, 0px));
		}
	}
</style>
