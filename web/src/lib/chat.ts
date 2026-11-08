// Chat presence noise reduction (pure, unit-tested). The server still emits every
// joined/left event; the client hides reconnect blips and folds runs into one line.
import type { ChatMessage } from './types';

export const RECONNECT_MS = 60_000;

export type ChatItem =
	| { kind: 'msg'; key: number; m: ChatMessage }
	| { kind: 'presence'; key: number; at: number; text: string };

const isPresence = (m: ChatMessage) => m.kind === 'event' && (m.text === 'joined' || m.text === 'left');

const names = (xs: string[]) => [...new Set(xs)].join(', ');

/**
 * 1. A "left" followed by a "joined" from the same name within 60 s is a reconnect: both hidden.
 * 2. Runs of consecutive joined/left events become one quiet line: "Ana, Sam joined · Priya left".
 */
export function collapsePresence(msgs: ChatMessage[]): ChatItem[] {
	const hidden = new Set<number>();
	const openLeft = new Map<string, ChatMessage>(); // name -> latest unmatched "left"
	for (const m of msgs) {
		if (!isPresence(m)) continue;
		if (m.text === 'left') openLeft.set(m.name, m);
		else {
			const l = openLeft.get(m.name);
			if (l && m.at - l.at <= RECONNECT_MS) {
				hidden.add(l.id);
				hidden.add(m.id);
			}
			openLeft.delete(m.name);
		}
	}

	const out: ChatItem[] = [];
	let run: ChatMessage[] = [];
	const flush = () => {
		if (!run.length) return;
		const joined = names(run.filter((m) => m.text === 'joined').map((m) => m.name));
		const left = names(run.filter((m) => m.text === 'left').map((m) => m.name));
		const text = [joined && `${joined} joined`, left && `${left} left`].filter(Boolean).join(' · ');
		out.push({ kind: 'presence', key: run[0].id, at: run.at(-1)!.at, text });
		run = [];
	};
	for (const m of msgs) {
		if (hidden.has(m.id)) continue;
		if (isPresence(m)) run.push(m);
		else {
			flush();
			out.push({ kind: 'msg', key: m.id, m });
		}
	}
	flush();
	return out;
}
