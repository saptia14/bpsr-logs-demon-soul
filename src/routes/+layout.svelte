<script lang="ts">
	import '../app.css';
	import { SETTINGS } from '$lib/settings-store';
	import { commands } from '$lib/bindings';
	import { onMount } from 'svelte';
	import Footer from '$lib/components/footer.svelte';
	import Header from '$lib/components/header.svelte';
	import { setupShortcuts } from '$lib/shortcuts';
	import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';

	let { children } = $props();
	let screenshotDiv: HTMLDivElement | undefined = $state();

	function applyTheme() {
		const theme = SETTINGS.accessibility.state.theme;
		if (theme === 'light') {
			document.documentElement.classList.add('light');
		} else {
			document.documentElement.classList.remove('light');
		}
	}

	// Apply theme on mount
	onMount(() => {
		applyTheme();
	});

	// Apply theme when it changes
	$effect(() => {
		applyTheme();
	});

	// Keep the backend integration flags in sync with the persisted settings.
	// The layout is always mounted (it wraps every page), so this is the single
	// source of truth — unlike the settings page, which only syncs while open.
	// These effects read the RuneStore's reactive state, so they run once the
	// store has loaded from disk (correcting the backend's startup defaults) and
	// again on every later toggle. We push unconditionally rather than diffing
	// against a previous value, so a persisted `false` is still propagated even
	// when it matches the backend's default — this is what fixes the webhook
	// firing when the UI shows it disabled.
	$effect(() => {
		commands.setBptimerEnabled(SETTINGS.integration.state.bptimer).catch((err: unknown) => {
			console.error('Failed to sync bptimer enabled state:', err);
		});
	});

	$effect(() => {
		commands.setWebhookEnabled(SETTINGS.integration.state.webhookEnabled).catch((err: unknown) => {
			console.error('Failed to sync webhook enabled state:', err);
		});
	});

	$effect(() => {
		commands
			.setGuildRelay(SETTINGS.integration.state.guildChatRelayEnabled)
			.catch((err: unknown) => {
				console.error('Failed to sync guild chat relay state:', err);
			});
	});

	$effect.pre(() => {
		(async () => {
			await setupShortcuts();
		})();
	});

	// TODO: workaround, need to wait for svelte tanstack devs to respond
	onMount(() => {
		const interval = setInterval(refreshWindow, 5 * 60 * 1000); // refresh every 5m
		return () => clearInterval(interval);
	});
	function refreshWindow() {
		window.location.reload();
	}

	const appWebview = getCurrentWebviewWindow();
	appWebview.listen<string>('navigate', (event) => {
		const route = event.payload;
		goto(route);
	});
	appWebview.listen('toggle-settings', () => {
		if (page.url.pathname === '/settings') {
			goto('/');
		} else {
			goto('/settings');
		}
	});
</script>

<svelte:window oncontextmenu={(e) => e.preventDefault()} />

<div
	class="flex h-screen flex-col text-sm text-foreground"
	style={`background-color: oklch(from var(--background) l c h / ${SETTINGS.accessibility.state.transparencyOpacity / 100});`}
	bind:this={screenshotDiv}
>
	<Header {screenshotDiv} />
	<main class="flex-1 overflow-y-auto">
		{@render children()}
	</main>
	<Footer />
</div>

<style>
	:global {
		/* Hide scrollbars globally but keep scrolling functional */
		* {
			-ms-overflow-style: none; /* IE and Edge */
			scrollbar-width: none; /* Firefox */
		}
		*::-webkit-scrollbar {
			display: none; /* Chrome, Safari, Edge */
		}
		/* Make body and html transparent for live window to allow window transparency */
		html,
		body {
			background: transparent !important;
		}
	}
</style>
