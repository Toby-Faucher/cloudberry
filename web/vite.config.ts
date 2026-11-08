import { defineConfig } from 'vitest/config';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';

const target = 'http://localhost:8080';

export default defineConfig({
	plugins: [
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			// SPA: the Rust server serves build/ and falls back to index.html.
			adapter: adapter({ fallback: 'index.html' })
		})
	],
	server: {
		proxy: {
			'/api': target,
			'/ws': { target, ws: true },
			'/audio': target,
			'/art': target
		}
	},
	test: {
		expect: { requireAssertions: true },
		projects: [
			{
				extends: './vite.config.ts',
				test: {
					name: 'unit',
					environment: 'node',
					include: ['src/**/*.{test,spec}.{js,ts}']
				}
			}
		]
	}
});
