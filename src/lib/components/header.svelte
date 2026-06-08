<script lang="ts">
	import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

	import CameraIcon from 'virtual:icons/lucide/camera';
	import TimerResetIcon from 'virtual:icons/lucide/timer-reset';
	import PauseIcon from 'virtual:icons/lucide/pause';
	import PlayIcon from 'virtual:icons/lucide/play';
	import MinusIcon from 'virtual:icons/lucide/minus';
	import XIcon from 'virtual:icons/lucide/x';
	import PointerIcon from 'virtual:icons/lucide/pointer';
	import SettingsIcon from 'virtual:icons/lucide/settings';
	import HomeIcon from 'virtual:icons/lucide/home';

	import { onMount, tick } from 'svelte';
	import { commands, type HeaderInfo } from '$lib/bindings';
	import { takeScreenshot, tooltip, getScreenshotBytes } from '$lib/utils.svelte';
	import { listen } from '@tauri-apps/api/event';
	import AbbreviatedNumber from '$lib/components/abbreviated-number.svelte';
	import { SETTINGS } from '$lib/settings-store';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';

	onMount(() => {
		fetchData();
		const interval = setInterval(fetchData, 200);

		const unlistenPromise = listen('request-screenshot', async () => {
			const bytes = await getScreenshotBytes(screenshotDiv);
			if (bytes) {
				await commands.submitPendingWebhook(Array.from(bytes));
			}
		});

		return () => {
			clearInterval(interval);
			unlistenPromise.then((unlisten) => unlisten());
		};
	});

	async function fetchData() {
		try {
			headerInfo = await commands.getHeaderInfo();
			if (
				SETTINGS.general.state.resetElapsed &&
				headerInfo.timeLastCombatPacketMs > 0 &&
				Date.now() - headerInfo.timeLastCombatPacketMs > SETTINGS.general.state.resetElapsed * 1000
			) {
				const bytes = await getScreenshotBytes(screenshotDiv);
				if (bytes) {
					await commands.resetEncounterWithImage(Array.from(bytes));
				} else {
					await commands.resetEncounter();
				}
				headerInfo = await commands.getHeaderInfo();
			}
		} catch (e) {
			console.error('Error fetching data: ', e);
		}
	}

	function formatElapsed(msElapsed: number) {
		const totalSeconds = Math.floor(Number(msElapsed) / 1000);
		const minutes = Math.floor((totalSeconds % 3600) / 60);
		const seconds = totalSeconds % 60;
		return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
	}

	let headerInfo: HeaderInfo = $state({
		totalDps: 0,
		totalDmg: 0,
		elapsedMs: 0,
		timeLastCombatPacketMs: 0
	});
	let isEncounterPaused = $state(false);
	let { screenshotDiv }: { screenshotDiv?: HTMLElement } = $props();
	const appWindow = getCurrentWebviewWindow();

	const isSettingsActive = $derived(page.url.pathname === '/settings');
	const opacity = $derived(SETTINGS.accessibility.state.transparencyOpacity / 100);

	// "Live" when combat happened recently (within the last ~3s).
	const isLive = $derived(
		!isEncounterPaused &&
			headerInfo.timeLastCombatPacketMs > 0 &&
			Date.now() - headerInfo.timeLastCombatPacketMs < 3000
	);

	function toggleSettings() {
		goto(isSettingsActive ? '/' : '/settings');
	}
</script>

<header
	data-tauri-drag-region
	class="hud-titlebar"
	style={`background: oklch(from var(--bg-2) l c h / ${opacity});`}
>
	<!-- Brand -->
	<span class="hud-brand">
		<span class="hud-brand-mark">
			<svg
				width="13"
				height="13"
				viewBox="0 0 24 24"
				fill="none"
				stroke="#04181c"
				stroke-width="3"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="M3 12h4l2.5 7 5-14L17 12h4" />
			</svg>
		</span>
		<span class="hud-brand-name">BPSR<b> Logs</b></span>
	</span>

	<!-- Live combat readout -->
	<span class="hud-combat">
		<span class="hud-combat-timer" {@attach tooltip(() => 'Time Elapsed')}>
			<span class="hud-live-dot" class:paused={!isLive}></span>
			{formatElapsed(headerInfo.elapsedMs)}
		</span>
		<span class="hud-tstat" {@attach tooltip(() => headerInfo.totalDmg.toLocaleString())}>
			<span class="k">T.DMG</span>
			<span class="v"><AbbreviatedNumber num={Number(headerInfo.totalDmg)} /></span>
		</span>
		<span class="hud-tstat" {@attach tooltip(() => headerInfo.totalDps.toLocaleString())}>
			<span class="k">T.DPS</span>
			<span class="v"><AbbreviatedNumber num={headerInfo.totalDps} /></span>
		</span>
	</span>

	<span class="flex-1"></span>

	<!-- Window tools -->
	<span class="hud-tools">
		<button
			class="hud-tool"
			onclick={async () => {
				await tick();
				await takeScreenshot(screenshotDiv);
			}}
			{@attach tooltip(() => 'Screenshot to Clipboard')}
		>
			<CameraIcon class="size-4" />
		</button>
		<button
			class="hud-tool"
			onclick={async () => {
				await tick();
				const bytes = await getScreenshotBytes(screenshotDiv);
				if (bytes) {
					await commands.resetEncounterWithImage(Array.from(bytes));
				} else {
					await commands.resetEncounter();
				}
				headerInfo = await commands.getHeaderInfo();
			}}
			{@attach tooltip(() => 'Reset Encounter')}
		>
			<TimerResetIcon class="size-4" />
		</button>
		<button
			class="hud-tool"
			class:on={isEncounterPaused}
			onclick={() => {
				commands.togglePauseEncounter();
				isEncounterPaused = !isEncounterPaused;
			}}
		>
			{#if isEncounterPaused}
				<PlayIcon class="size-4" {@attach tooltip(() => 'Resume Encounter')} />
			{:else}
				<PauseIcon class="size-4" {@attach tooltip(() => 'Pause Encounter')} />
			{/if}
		</button>
		<button
			class="hud-tool"
			onclick={() => appWindow.setIgnoreCursorEvents(true)}
			{@attach tooltip(() => 'Click-through')}
		>
			<PointerIcon class="size-4" />
		</button>
		<span class="div"></span>
		<button
			class="hud-tool"
			class:on={isSettingsActive}
			onclick={toggleSettings}
			{@attach tooltip(() => (isSettingsActive ? 'Home' : 'Settings'))}
		>
			{#if isSettingsActive}
				<HomeIcon class="size-4" />
			{:else}
				<SettingsIcon class="size-4" />
			{/if}
		</button>
		<span class="div"></span>
		<button
			class="hud-tool"
			onclick={() => appWindow.hide()}
			{@attach tooltip(() => 'Minimize')}
		>
			<MinusIcon class="size-4" />
		</button>
		<button class="hud-tool danger" onclick={() => commands.quitApp()} {@attach tooltip(() => 'Quit')}>
			<XIcon class="size-4" />
		</button>
	</span>
</header>
