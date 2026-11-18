import { sveltekit } from '@sveltejs/kit/vite';
import { autoPort } from '@bridgerb/port-from-name';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit(), autoPort()]
});
