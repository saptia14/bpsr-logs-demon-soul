import tippy from 'tippy.js';
import 'tippy.js/dist/tippy.css';
import type { Attachment } from 'svelte/attachments';
import html2canvas from 'html2canvas-pro';
import { writeText, writeImage } from '@tauri-apps/plugin-clipboard-manager';
import { image } from '@tauri-apps/api';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

// Tactical HUD class colors (from design/project/bpsr/tokens.css).
// Each resolves to a CSS variable defined in app.css.
export const classColors: Record<string, string> = {
	Marksman: 'var(--c-red)',
	Stormblade: 'var(--c-purple)',
	'Frost Mage': 'var(--c-blue)',
	'Wind Knight': 'var(--c-cyan)',
	'Twin Striker': 'var(--c-gold)',
	'Beat Performer': 'var(--c-green)',
	'Verdant Oracle': 'var(--c-emerald)',
	'Heavy Guardian': 'var(--c-brown)',
	'Shield Knight': 'var(--c-amber)'
};

/** Solid class color (used as the row `--rc` and bar accent). */
export function getClassColor(className: string): string {
	return classColors[className] ?? 'var(--tx-2)';
}

export function getClassIcon(className: string): string {
	if (
		className === 'Hidden Class' ||
		className === 'Unknown Class' ||
		className === 'Undefined Class' ||
		!className
	) {
		return '/images/blank.png';
	}
	return `/images/classes/${className}.png`;
}

import SkillIconJson from '$lib/data/json/SkillIcon.json';
export const SkillIconMap: Record<string, string> = SkillIconJson;
export function getSkillIcon(skillUid: number): string {
	const key = skillUid.toString();
	if (key in SkillIconMap) {
		return `/images/skills/${SkillIconMap[key]}.webp`;
	} else {
		return '/images/blank.png';
	}
}

// https://svelte.dev/docs/svelte/@attach#Attachment-factories
export function tooltip(getContent: () => string): Attachment {
	return (element: Element) => {
		const tooltip = tippy(element, {
			content: ''
		});
		$effect(() => {
			tooltip.setContent(getContent());
		});
		return tooltip.destroy;
	};
}

export async function copyToClipboard(
	error: MouseEvent & { currentTarget: EventTarget & HTMLElement },
	content: string
) {
	// TODO: add a way to simulate a "click" animation
	error.stopPropagation();
	await writeText(content);
}

export async function takeScreenshot(target?: HTMLElement): Promise<void> {
	if (!target) return;
	// Give the browser a paint frame (helps if caller just changed DOM)
	await new Promise(requestAnimationFrame);

	const canvas = await html2canvas(target, { backgroundColor: '#27272A' });

	const blob: Blob | null = await new Promise((resolve) => canvas.toBlob(resolve));
	if (!blob) return;

	try {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any -- Image type mismatch between @tauri-apps/api and @tauri-apps/plugin-clipboard-manager
		await writeImage((await image.Image.fromBytes(await blob.arrayBuffer())) as any);
	} catch (error) {
		console.error('Failed to take a screenshot', error);
	}
}

export async function getScreenshotBytes(target?: HTMLElement): Promise<Uint8Array | null> {
	if (!target) return null;
	await new Promise(requestAnimationFrame);

	const canvas = await html2canvas(target, { backgroundColor: '#27272A' });
	const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'));
	if (!blob) return null;
	const buffer = await blob.arrayBuffer();
	return new Uint8Array(buffer);
}

let isClickthrough = false;

export async function setClickthrough(bool: boolean) {
	const liveWindow = await WebviewWindow.getByLabel('live');
	await liveWindow?.setIgnoreCursorEvents(bool);
	isClickthrough = bool;
}

export async function toggleClickthrough() {
	const liveWindow = await WebviewWindow.getByLabel('live');
	await liveWindow?.setIgnoreCursorEvents(!isClickthrough);
	isClickthrough = !isClickthrough;
}
