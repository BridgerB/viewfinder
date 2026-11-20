<script lang="ts">
import PhotoFrame from "#lib/PhotoFrame.svelte";
import { compass, placeName } from "#lib/places.ts";
import ResultView from "#lib/ResultView.svelte";
import { resolve } from "$app/paths";
import { page } from "$app/state";
import type { PageProps } from "./$types";

let { data }: PageProps = $props();

const photoUrl = $derived(resolve("/p/[id]/photo", { id: data.id }));
const title = $derived(
	`${placeName([data.result.best.lat, data.result.best.lon])} · facing ${compass(data.result.best.yaw)}`,
);
</script>

<svelte:head>
	<title>{title} · Viewfinder</title>
	<!-- Unlisted: reachable by link only. -->
	<meta name="robots" content="noindex, nofollow" />
	<meta property="og:title" content={title} />
	<meta property="og:description" content="Where this mountain photo was taken, found from its skyline." />
	<meta property="og:image" content={new URL(photoUrl, page.url.origin).href} />
	<meta property="og:type" content="website" />
	<meta name="twitter:card" content="summary_large_image" />
</svelte:head>

<div class="banner">
	<span>Shared result · read-only</span>
	<a class="pill" href={resolve("/")}>Try your own photo</a>
</div>

{#snippet photo()}
	<PhotoFrame
		src={photoUrl}
		width={data.width}
		height={data.height}
		alt="A mountain skyline, with the traced ridge and the matching terrain horizon drawn on it"
		skyline={data.skyline}
		horizon={data.result.outline}
		box={data.box}
		boxMode="view"
	/>
{/snippet}

<ResultView result={data.result} height={data.height} {photo} shareUrl={page.url.href} />

<style>
	.banner {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		align-items: center;
		gap: 8px 16px;
		margin-top: 12px;
		padding: 12px 12px 12px 18px;
		border-radius: 99px;
		background: var(--soft);
		font-size: 14px;
	}

	.banner .pill {
		padding: 9px 18px;
	}
</style>
