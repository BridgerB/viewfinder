<script lang="ts">
	import { resolve } from '$app/paths';
</script>

<h1>Viewfinder</h1>

<ul>
	<li><a href={resolve('/timpanogos')}>Mt Timpanogos 360° Horizon</a></li>
</ul>

<style>
	:global(body) {
		margin: 0;
		padding: 40px;
		background: #0a0a0f;
		font-family: system-ui;
		color: #fff;
	}

	h1 {
		margin: 0 0 20px;
	}

	a {
		color: #4facfe;
	}

	a:hover {
		color: #00f2fe;
	}
</style>
