/**
 * Synced lyrics from LRCLIB (https://lrclib.net): clean the YouTube title into a
 * search query, take the first hit that has `syncedLyrics`, parse the LRC.
 */

export interface LrcLine {
	/** ms into the track */
	t: number;
	text: string;
}

const NOISE =
	/official|video|audio|lyrics?|visuali[sz]er|music|hd|hq|4k|mv|m\/v|remaster(ed)?|explicit|clip|live|color coded|full album|topic/i;

/**
 * YouTube title -> LRCLIB query. Drops bracketed noise ("(Official Video)",
 * "[Lyrics]", "【MV】"), featured-artist tails ("ft. X", "feat. X"), "| extra" and
 * trailing "- Topic"; keeps meaningful brackets like "(Remix)" or "(Acoustic)".
 */
export function cleanTitle(title: string): string {
	return title
		.replace(/[([【{]([^)\]】}]*)[)\]】}]/g, (m, inner: string) => (NOISE.test(inner) ? ' ' : m))
		.replace(/\s(ft\.?|feat\.?|featuring)\s[^-–—|([]*/gi, ' ')
		.replace(/\s[|｜].*$/, '')
		.replace(/\s[-–—]\s*topic$/i, '')
		.replace(/["“”]/g, '')
		.replace(/\s+/g, ' ')
		.replace(/^[\s\-–—]+|[\s\-–—]+$/g, '');
}

const STAMP = /\[(\d+):(\d{1,2}(?:[.:]\d{1,3})?)\]/g;

/** Parse LRC text. Supports multiple stamps per line and the [offset:±ms] tag. Sorted by time. */
export function parseLrc(lrc: string): LrcLine[] {
	const off = Number(/\[offset:\s*([+-]?\d+)\s*\]/i.exec(lrc)?.[1] ?? 0);
	const out: LrcLine[] = [];
	for (const raw of lrc.split(/\r?\n/)) {
		const stamps = [...raw.matchAll(STAMP)];
		if (!stamps.length) continue;
		const text = raw.replace(STAMP, '').trim();
		for (const [, m, s] of stamps) {
			// Positive offset = lyrics shown earlier.
			const t = Math.round((Number(m) * 60 + Number(s.replace(':', '.'))) * 1000) - off;
			out.push({ t: Math.max(0, t), text });
		}
	}
	return out.sort((a, b) => a.t - b.t);
}

/** Index of the line being sung at `posMs` (last line with t <= pos), or -1 before the first. */
export function lineAt(lines: readonly LrcLine[], posMs: number): number {
	let lo = 0;
	let hi = lines.length - 1;
	let ans = -1;
	while (lo <= hi) {
		const mid = (lo + hi) >> 1;
		if (lines[mid].t <= posMs) {
			ans = mid;
			lo = mid + 1;
		} else hi = mid - 1;
	}
	return ans;
}

// Per track id, for the life of the page. Failures cache as null (silently hidden).
const cache = new Map<string, Promise<LrcLine[] | null>>();

export function fetchLyrics(id: string, title: string): Promise<LrcLine[] | null> {
	let p = cache.get(id);
	if (!p) {
		const q = cleanTitle(title);
		p = fetch(`https://lrclib.net/api/search?q=${encodeURIComponent(q)}`)
			.then((r) => (r.ok ? r.json() : []))
			.then((hits: { syncedLyrics?: string | null }[]) => {
				const synced = Array.isArray(hits) ? hits.find((h) => h.syncedLyrics)?.syncedLyrics : null;
				const lines = synced ? parseLrc(synced) : [];
				return lines.length ? lines : null;
			})
			.catch(() => null);
		cache.set(id, p);
	}
	return p;
}
