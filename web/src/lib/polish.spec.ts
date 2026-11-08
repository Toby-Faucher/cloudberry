import { describe, it, expect } from 'vitest';
import { cleanTitle, lineAt, parseLrc } from './lyrics';
import { hue, isYouTubeUrl, trackTint } from './util';

describe('cleanTitle (LRCLIB query)', () => {
	it.each([
		['Rick Astley - Never Gonna Give You Up (Official Music Video)', 'Rick Astley - Never Gonna Give You Up'],
		['Daft Punk - Get Lucky [Lyrics] ft. Pharrell Williams', 'Daft Punk - Get Lucky'],
		['Calvin Harris feat. Rihanna - This Is What You Came For (Official Video)', 'Calvin Harris - This Is What You Came For'],
		['Song Name (Remix) | Some Channel', 'Song Name (Remix)'],
		['Artist - "Quoted" 【MV】 [HD]', 'Artist - Quoted'],
		['Bonobo - Kerala (Official Audio)', 'Bonobo - Kerala'],
		['Plain Title', 'Plain Title']
	])('%s', (input, want) => {
		expect(cleanTitle(input)).toBe(want);
	});
});

describe('parseLrc', () => {
	it('parses stamps, skips tags, sorts, handles repeated lines and offset', () => {
		const lrc = [
			'[ar:Someone]',
			'[ti:Song]',
			'[00:12.50]Second line',
			'[00:01.00][00:20.000]Chorus',
			'[01:02.3]',
			'not a lyric line'
		].join('\n');
		expect(parseLrc(lrc)).toEqual([
			{ t: 1000, text: 'Chorus' },
			{ t: 12500, text: 'Second line' },
			{ t: 20000, text: 'Chorus' },
			{ t: 62300, text: '' }
		]);
	});

	it('applies [offset:+ms] (positive = earlier)', () => {
		expect(parseLrc('[offset:+500]\n[00:02.00]hi')).toEqual([{ t: 1500, text: 'hi' }]);
	});

	it('finds the current line', () => {
		const lines = parseLrc('[00:01.00]a\n[00:05.00]b\n[00:09.00]c');
		expect(lineAt(lines, 0)).toBe(-1);
		expect(lineAt(lines, 1000)).toBe(0);
		expect(lineAt(lines, 8999)).toBe(1);
		expect(lineAt(lines, 60_000)).toBe(2);
		expect(lineAt([], 5)).toBe(-1);
	});
});

describe('avatar hue', () => {
	it('is deterministic and stays in the warm band', () => {
		const names = ['Toby', 'Tobi', 'Ana', 'Sam', 'Priya', 'Jürgen', 'x', ''];
		for (const n of names) {
			const h = hue(n);
			expect(hue(n)).toBe(h);
			expect(h >= 340 || h < 120).toBe(true); // 340..360 ∪ 0..120
		}
		expect(hue(' Toby ')).toBe(hue('toby')); // same person, same colour
		expect(hue('Toby')).not.toBe(hue('Tobi'));
	});
});

describe('isYouTubeUrl', () => {
	it('accepts the server hosts only', () => {
		expect(isYouTubeUrl('https://youtu.be/dQw4w9WgXcQ')).toBe(true);
		expect(isYouTubeUrl(' https://music.youtube.com/watch?v=dQw4w9WgXcQ ')).toBe(true);
		expect(isYouTubeUrl('https://evilyoutube.com/watch?v=x')).toBe(false);
		expect(isYouTubeUrl('javascript://youtube.com/%0aalert(1)')).toBe(false);
		expect(isYouTubeUrl('hello')).toBe(false);
	});
});

describe('trackTint', () => {
	it('keeps brand orange orange and lifts greys into a usable tint', () => {
		const rgb = (s: string) => s.match(/\d+/g)!.map(Number);
		rgb(trackTint([240, 145, 43])).forEach((v, i) => expect(Math.abs(v - [240, 145, 43][i])).toBeLessThan(8));
		const grey = trackTint([128, 128, 128]);
		const [r, g, b] = grey.match(/\d+/g)!.map(Number);
		expect(r).toBeGreaterThan(b + 80); // saturated, warm
		expect(trackTint([0, 0, 0])).not.toBe('rgb(0 0 0)');
	});
});
