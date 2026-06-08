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

	// Union (Guild) = 4 is the default focus. Colors from the HUD channel palette.
	const ALL_CHANNELS: { id: number; label: string; color: string }[] = [
		{ id: 4, label: 'Union', color: 'rgb(255,214,0)' },
		{ id: 1, label: 'World', color: 'rgb(100,199,255)' },
		{ id: 2, label: 'Local', color: 'rgb(143,237,143)' },
		{ id: 3, label: 'Team', color: 'rgb(255,181,194)' },
		{ id: 6, label: 'Group', color: 'rgb(173,217,230)' },
		{ id: 99, label: 'System', color: 'rgb(255,99,71)' }
	];
	const colorOf = (channel: number) => ALL_CHANNELS.find((c) => c.id === channel)?.color ?? 'var(--tx-1)';

	let selected = $state<number[]>([4]);
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
		return new Date(unixSecs * 1000).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
	}
</script>

<div class="flex h-full flex-col hud-anim">
	<!-- Channel filter row -->
	<div class="hud-chat-channels flex items-center gap-1 px-3 pt-3 pb-2">
		{#each ALL_CHANNELS as ch (ch.id)}
			<button
				class="hud-cch"
				class:on={selected.includes(ch.id)}
				style={selected.includes(ch.id) ? `color:${ch.color}` : ''}
				onclick={() => toggleChannel(ch.id)}
			>
				<span class="cdot" style={`background:${ch.color}`}></span>{ch.label}
			</button>
		{/each}
		<span class="flex-1"></span>
		<button class="hud-tool" class:on={showSettings} onclick={() => { showSettings = !showSettings; if (showSettings) refreshStatus(); }} aria-label="Relay settings">
			<SettingsIcon class="size-4" />
		</button>
		<button class="hud-tool danger" onclick={() => commands.clearChat().then(fetchData)} aria-label="Clear chat log">
			<TrashIcon class="size-4" />
		</button>
	</div>

	{#if showSettings}
		<div class="hud-scard" style="margin:0 12px 8px">
			<div class="hud-srow" style="padding-top:0">
				<div class="si">
					<div class="lab">Relay Guild (Union) chat to Discord</div>
					<div class="des">Posts <span style="font-family:var(--mono)">Name: text</span> via the shared dedupe server — one copy reaches Discord even if several guildmates run this.</div>
				</div>
				<button
					class="hud-tgl"
					class:on={SETTINGS.integration.state.guildChatRelayEnabled}
					aria-label="Toggle guild relay"
					onclick={() => (SETTINGS.integration.state.guildChatRelayEnabled = !SETTINGS.integration.state.guildChatRelayEnabled)}
				></button>
			</div>
			<div class="hud-srow" style="padding-bottom:0">
				<div class="si flex items-center gap-3 text-tiny">
					<span class="flex items-center gap-1.5">
						<span class="cdot" style={`width:7px;height:7px;border-radius:99px;background:${relayStatus.configured ? 'var(--good)' : 'var(--bad)'}`}></span>
						API key {relayStatus.configured ? 'configured' : 'missing'}
					</span>
					<span class="flex items-center gap-1.5">
						<span class="cdot" style={`width:7px;height:7px;border-radius:99px;background:${relayStatus.reachable ? 'var(--good)' : 'var(--bad)'}`}></span>
						Server {relayStatus.reachable ? 'reachable' : 'unreachable'}
					</span>
				</div>
				<button class="hud-gbtn" onclick={refreshStatus}>Recheck</button>
			</div>
		</div>
	{/if}

	<!-- Messages -->
	<div bind:this={scrollEl} class="hud-chat-body min-h-0 flex-1 overflow-y-auto py-1">
		{#if messages.length === 0}
			<div class="hud-empty"><p>No messages yet for the selected channel(s).</p></div>
		{:else}
			{#each messages as m (m.id)}
				<div class="hud-msg">
					<span class="ts">{fmtTime(m.timestamp)}</span>
					<span class="body">
						<button
							type="button"
							class="au cursor-pointer border-none bg-transparent p-0"
							style={`color:${colorOf(m.channel)}`}
							onclick={(e) => copyToClipboard(e, m.senderName)}
							title={`#${m.senderUid} · Lv${m.senderLevel} · ${m.channelName}`}
						>{m.senderName}</button>{m.text}
					</span>
				</div>
			{/each}
		{/if}
	</div>
</div>
