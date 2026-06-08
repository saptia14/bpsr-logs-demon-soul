<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { getVersion } from '@tauri-apps/api/app';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { SETTINGS } from '$lib/settings-store';

	import ActivityIcon from 'virtual:icons/lucide/activity';
	import HeartIcon from 'virtual:icons/lucide/heart';
	import LayoutGridIcon from 'virtual:icons/lucide/layout-grid';
	import HistoryIcon from 'virtual:icons/lucide/history';
	import MessageSquareIcon from 'virtual:icons/lucide/message-square';

	const tabs = [
		{ id: 'dps', label: 'DPS', path: '/', icon: ActivityIcon },
		{ id: 'heal', label: 'Heal', path: '/heal', icon: HeartIcon },
		{ id: 'modules', label: 'Modules', path: '/optimizer', icon: LayoutGridIcon },
		{ id: 'history', label: 'History', path: '/history', icon: HistoryIcon },
		{ id: 'chat', label: 'Chat', path: '/chat', icon: MessageSquareIcon }
	];

	function isActive(id: string): boolean {
		const p = page.url.pathname;
		if (id === 'dps') return p === '/' || p.startsWith('/skills');
		if (id === 'heal') return p.startsWith('/heal');
		if (id === 'modules') return p.startsWith('/optimizer');
		if (id === 'history') return p.startsWith('/history');
		if (id === 'chat') return p.startsWith('/chat');
		return false;
	}

	const opacity = $derived(SETTINGS.accessibility.state.transparencyOpacity / 100);
</script>

<footer
	class="hud-tabbar"
	style={`background: oklch(from var(--bg-1) l c h / ${opacity});`}
>
	{#each tabs as tab (tab.id)}
		{@const Icon = tab.icon}
		<button class="hud-tab" class:active={isActive(tab.id)} onclick={() => goto(tab.path)}>
			<Icon class="size-3.5" />
			{tab.label}
		</button>
	{/each}
	<span class="flex-1"></span>
	<button
		type="button"
		class="hud-tabbar-ver"
		onclick={() => openUrl('https://discord.gg/6zXd7hf9KY')}
		aria-label="Open DemonSoul Discord"
	>
		<span class="pip"></span>DemonSoul v{#await getVersion()}0.0.0{:then version}{version}{/await}
	</button>
</footer>
