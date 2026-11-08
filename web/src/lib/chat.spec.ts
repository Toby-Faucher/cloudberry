import { describe, expect, it } from 'vitest';
import { collapsePresence, type ChatItem } from './chat';
import type { ChatMessage } from './types';

let id = 0;
const ev = (name: string, text: string, at: number): ChatMessage => ({ id: ++id, kind: 'event', name, text, at });
const say = (name: string, text: string, at: number): ChatMessage => ({ id: ++id, kind: 'user', name, text, at });
const show = (items: ChatItem[]) => items.map((i) => (i.kind === 'presence' ? `~ ${i.text}` : `${i.m.name}: ${i.m.text}`));

describe('collapsePresence', () => {
	it('folds consecutive joins and leaves into one line', () => {
		const items = collapsePresence([
			ev('Ana', 'joined', 0),
			ev('Sam', 'joined', 1000),
			ev('Priya', 'left', 2000),
			say('Ana', 'hi', 3000),
			ev('Sam', 'joined', 400_000) // a separate run after a message
		]);
		expect(show(items)).toEqual(['~ Ana, Sam joined · Priya left', 'Ana: hi', '~ Sam joined']);
	});

	it('hides a left + joined pair by the same name within 60 s (a reconnect)', () => {
		const items = collapsePresence([
			say('Ana', 'brb', 0),
			ev('Sam', 'left', 10_000),
			say('Ana', 'still here', 20_000),
			ev('Sam', 'joined', 69_000),
			say('Sam', 'back', 70_000)
		]);
		expect(show(items)).toEqual(['Ana: brb', 'Ana: still here', 'Sam: back']);
	});

	it('keeps a leave and a much later rejoin', () => {
		const items = collapsePresence([ev('Sam', 'left', 0), ev('Sam', 'joined', 61_000)]);
		expect(show(items)).toEqual(['~ Sam joined · Sam left']);
	});

	it('only pairs a join with an earlier leave, and dedupes names', () => {
		const items = collapsePresence([
			ev('Ana', 'joined', 0),
			ev('Ana', 'left', 5000),
			ev('Ana', 'joined', 9000), // hides with the left before it
			ev('Bo', 'joined', 9500),
			ev('Bo', 'joined', 9600)
		]);
		expect(show(items)).toEqual(['~ Ana, Bo joined']);
	});

	it('leaves other events and user messages alone', () => {
		const items = collapsePresence([ev('Cloudberry', 'paused', 0), say('Ana', 'joined', 1)]);
		expect(show(items)).toEqual(['Cloudberry: paused', 'Ana: joined']);
	});
});
