#!/usr/bin/env node
// Regenerate the README screenshots (docs/assets/screenshot-{desktop,mobile}.png).
// Builds nothing itself. Start these first, in separate terminals (restart the mock for a clean chat):
//
//   cd web && node scripts/mock-server.mjs     # fake station on :8080 (no cargo/yt-dlp needed)
//   cd web && npm run dev                       # Vite on :5173, proxies to :8080
//   ./scripts/build-wasm.sh                     # optional: real WASM visualizer (else JS fallback)
//
// Then, from the repo root (playwright-core is fetched by npx, not added to package.json):
//
//   npx --yes -p playwright-core@1.56 -c 'node scripts/screenshots.mjs'
//
// Env: BASE_URL (default http://localhost:5173), CHROME (path to a Chromium binary; default:
// newest ~/.cache/ms-playwright/chromium-*, else playwright's own lookup).
import { createRequire } from 'node:module';
import { existsSync, readdirSync, realpathSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { homedir } from 'node:os';

const root = join(dirname(new URL(import.meta.url).pathname), '..');
const out = join(root, 'docs/assets');
const base = process.env.BASE_URL ?? 'http://localhost:5173';

// npx puts playwright-core's bin on PATH but not on Node's module path; resolve it from there.
async function loadPlaywright() {
	try {
		return await import('playwright-core');
	} catch {
		for (const dir of (process.env.PATH ?? '').split(':')) {
			const bin = join(dir, 'playwright-core');
			if (existsSync(bin)) return createRequire(realpathSync(bin))('playwright-core');
		}
		throw new Error("playwright-core not found; run via: npx --yes -p playwright-core@1.56 -c 'node scripts/screenshots.mjs'");
	}
}

function findChrome() {
	if (process.env.CHROME) return process.env.CHROME;
	const cache = join(homedir(), '.cache/ms-playwright');
	if (!existsSync(cache)) return undefined;
	const dirs = readdirSync(cache)
		.filter((d) => /^chromium-\d+$/.test(d))
		.sort((a, b) => Number(b.split('-')[1]) - Number(a.split('-')[1]));
	for (const d of dirs) {
		const p = join(cache, d, 'chrome-linux64/chrome');
		if (existsSync(p)) return p;
	}
	return undefined;
}

const { chromium } = await loadPlaywright();
const browser = await chromium.launch({
	executablePath: findChrome(),
	args: ['--autoplay-policy=no-user-gesture-required']
});

async function tuneIn(name, context) {
	const ctx = await browser.newContext({ colorScheme: 'dark', ...context });
	const page = await ctx.newPage();
	page.on('pageerror', (e) => console.error(`[${name}] ${e.message}`));
	await page.goto(base);
	// Headless Chromium on a bare Linux box often has no colour emoji font (reactions render as
	// tofu); append Noto Color Emoji to the app's font tokens, for the screenshot only.
	await page.addStyleTag({
		content: `@import url('https://fonts.googleapis.com/css2?family=Noto+Color+Emoji&display=block');
		html:root {
			--font-ui: 'Host Grotesk Variable', 'Helvetica Neue', Arial, 'Noto Color Emoji', sans-serif;
		}`
	});
	await page.fill('#name', name);
	await page.click('button[type=submit]');
	await page.click('text=Tune in');
	// The pause button only appears once the player is running on the shared timeline.
	await page.waitForSelector('[aria-label="Pause for everyone"]', { timeout: 15000 });
	return page;
}

try {
	// Both listeners stay connected, so each shot shows two people in the room.
	const shots = [
		['screenshot-desktop.png', await tuneIn('Toby', { viewport: { width: 1440, height: 900 } })],
		[
			'screenshot-mobile.png',
			await tuneIn('Ana', { viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true })
		]
	];
	// Let the progress bar and visualizer settle into a mid-song frame.
	await shots[0][1].waitForTimeout(6000);
	for (const [file, page] of shots) {
		await page.evaluate(() => document.fonts.load('16px "Noto Color Emoji"', '🔥'));
		await page.screenshot({ path: join(out, file) });
		console.log(`wrote docs/assets/${file}`);
	}
} finally {
	await browser.close();
}
