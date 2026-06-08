<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { commands, type ChatRow, type GuildRelayStatus } from '$lib/bindings';
	import { SETTINGS } from '$lib/settings-store';
	import { copyToClipboard } from '$lib/utils.svelte';
	import SettingsIcon from 'virtual:icons/lucide/settings';
	import TrashIcon from 'virtual:icons/lucide/trash-2';

	let relayStatus = $state<GuildRelayStatus>({ configured: false, reachable: false });

	async function refreshStatus() {
		try {
			relayStatus = await commands.getGuildRelayStatus();
		} catch (e) {
			console.error('Failed to read relay status:', e);
		}
	}

	// Channels to display. Union (Guild) = 4 is the default focus.
	const ALL_CHANNELS: { id: number; label: string; color: string }[] = [
		{ id: 4, label: 'Union', color: 'rgb(255,214,0)' },
		{ id: 1, label: 'World', color: 'rgb(100,199,255)' },
		{ id: 2, label: 'Local', color: 'rgb(143,237,143)' },
		{ id: 3, label: 'Team', color: 'rgb(255,181,194)' },
		{ id: 6, label: 'Group', color: 'rgb(173,217,230)' },
		{ id: 99, label: 'System', color: 'rgb(255,99,71)' }
	];
	const colorOf = (channel: number) =>
		ALL_CHANNELS.find((c) => c.id === channel)?.color ?? 'inherit';

	let selected = $state<number[]>([4]); // Union by default
	let messages = $state<ChatRow[]>([]);
	let showSettings = $state(false);
	let scrollEl: HTMLDivElement | undefined = $state();

	onMount(() => {
		fetchData();
		const interval = setInterval(fetchData, 500);
		return () => clearInterval(interval);
	});

	async function fetchData() {
		const prevLen = messages.length;
		messages = await commands.getChatMessages(selected, 300);
		if (messages.length !== prevLen) {
			await tick();
			scrollEl?.scrollTo({ top: scrollEl.scrollHeight });
		}
	}

	function toggleChannel(id: number) {
		selected = selected.includes(id) ? selected.filter((c) => c !== id) : [...selected, id];
		fetchData();
	}

	function fmtTime(unixSecs: number): string {
		return new Date(unixSecs * 1000).toLocaleTimeString(undefined, {
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	const opacity = $derived(SETTINGS.accessibility.state.transparencyOpacity / 100);
</script>

<div class="flex h-full flex-col text-xs">
	<!-- Toolbar -->
	<div
		class="sticky top-0 z-10 flex items-center gap-1 border-b px-1.5 py-1"
		style={`background-color: oklch(from var(--card) l c h / ${opacity});`}
	>
		{#each ALL_CHANNELS as ch (ch.id)}
			<button
				class="rounded px-1.5 py-0.5 font-medium transition-colors"
				style={selected.includes(ch.id) ? `background-color:${ch.color}33;color:${ch.color}` : ''}
				class:text-muted-foreground={!selected.includes(ch.id)}
				onclick={() => toggleChannel(ch.id)}
			>
				{ch.label}
			</button>
		{/each}
		<span class="ml-auto flex items-center gap-0.5">
			<button
				class="rounded p-1 hover:bg-accent"
				onclick={() => {
					showSettings = !showSettings;
					if (showSettings) refreshStatus();
				}}
				aria-label="Relay settings"
			>
				<SettingsIcon class="size-3.5" />
			</button>
			<button
				class="rounded p-1 text-muted-foreground hover:bg-destructive/15 hover:text-destructive"
				onclick={() => commands.clearChat().then(fetchData)}
				aria-label="Clear chat log"
			>
				<TrashIcon class="size-3.5" />
			</button>
		</span>
	</div>

	{#if showSettings}
		<!-- Guild (Union) -> Discord dedupe relay config -->
		<div
			class="space-y-2 border-b px-2 py-2"
			style={`background-color: oklch(from var(--card) l c h / ${opacity});`}
		>
			<label class="flex items-center justify-between gap-2">
				<span class="font-medium">Relay Guild (Union) chat to Discord</span>
				<input type="checkbox" bind:checked={SETTINGS.integration.state.guildChatRelayEnabled} />
			</label>
			<div class="flex items-center gap-3 text-tiny">
				<span class="flex items-center gap-1">
					<span
						class="inline-block size-2 rounded-full {relayStatus.configured
							? 'bg-green-500'
							: 'bg-red-500'}"
					></span>
					API key {relayStatus.configured ? 'configured' : 'missing'}
				</span>
				<span class="flex items-center gap-1">
					<span
						class="inline-block size-2 rounded-full {relayStatus.reachable
							? 'bg-green-500'
							: 'bg-red-500'}"
					></span>
					Server {relayStatus.reachable ? 'reachable' : 'unreachable'}
				</span>
				<button class="ml-auto rounded px-1.5 py-0.5 hover:bg-accent" onclick={refreshStatus}>
					Recheck
				</button>
			</div>
			<p class="text-tiny text-muted-foreground">
				Guild text posts as <span class="font-mono">Name: text</span> via the shared dedupe server, so
				only one copy reaches Discord even when several guildmates run this. Stickers, voice and images
				aren't relayed (the game provides no media URL for them).
			</p>
		</div>
	{/if}

	<!-- Messages -->
	<div bind:this={scrollEl} class="min-h-0 flex-1 overflow-y-auto px-2 py-1">
		{#if messages.length === 0}
			<p class="p-4 text-center text-muted-foreground">
				No messages yet for the selected channel(s).
			</p>
		{:else}
			{#each messages as m (m.id)}
				<div class="flex gap-1.5 py-0.5 leading-snug">
					<span class="shrink-0 text-tiny text-muted-foreground">{fmtTime(m.timestamp)}</span>
					<span class="break-words">
						<button
							type="button"
							class="cursor-pointer border-none bg-transparent p-0 font-semibold"
							style={`color:${colorOf(m.channel)}`}
							onclick={(e) => copyToClipboard(e, m.senderName)}
							title={`#${m.senderUid} · Lv${m.senderLevel} · ${m.channelName}`}
							>{m.senderName}</button
						><span class="text-muted-foreground">:</span>
						{m.text}
					</span>
				</div>
			{/each}
		{/if}
	</div>
</div>
