<script lang="ts">
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import SettingsButton from './settings-button.svelte';
	import { commands, type CaptureDiagnostics } from '$lib/bindings';
	import { onMount } from 'svelte';

	const SETTINGS_CATEGORY = 'capture';

	let diag = $state<CaptureDiagnostics>({ gameDetected: false, processCount: 0, ports: [] });

	onMount(() => {
		refresh();
		const interval = setInterval(refresh, 2000);
		return () => clearInterval(interval);
	});

	async function refresh() {
		try {
			diag = await commands.getCaptureDiagnostics();
		} catch (e) {
			console.error('Failed to read capture diagnostics:', e);
		}
	}
</script>

<Tabs.Content value={SETTINGS_CATEGORY} class="space-y-4">
	<Card.Root>
		<Card.Header>
			<Card.Title>Connection Status</Card.Title>
		</Card.Header>
		<Card.Content class="space-y-3 text-sm">
			<div class="flex items-center gap-2">
				<span
					class="inline-block size-2.5 rounded-full {diag.gameDetected
						? 'bg-green-500'
						: 'bg-red-500'}"
				></span>
				<span class="font-medium">
					{diag.gameDetected ? 'Game process detected' : 'Game process not detected'}
				</span>
				{#if diag.processCount > 1}
					<span class="text-muted-foreground">({diag.processCount} clients)</span>
				{/if}
			</div>

			<div>
				<p class="text-muted-foreground">Tracked game TCP ports</p>
				{#if diag.ports.length > 0}
					<div class="mt-1 flex flex-wrap gap-1">
						{#each diag.ports as port (port)}
							<span class="rounded bg-accent px-1.5 py-0.5 font-mono text-xs">{port}</span>
						{/each}
					</div>
				{:else}
					<p class="mt-1 text-xs text-muted-foreground">
						None yet — open the game and load into a zone.
					</p>
				{/if}
			</div>

			<p class="text-xs text-muted-foreground">
				Ports are read from the OS for the game process (BPSR / BPSR_STEAM / BPSR_EPIC) and used to
				identify the game's traffic — independent of which network adapter or IP a VPN uses.
			</p>

			<SettingsButton
				onclick={() => commands.hardReset()}
				buttonLabel="Restart"
				label="Restart Packet Capture"
				description="Reopen the capture handle. Use this if no data is appearing or after changing your VPN/network setup."
			/>
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>VPN & ExitLag</Card.Title>
		</Card.Header>
		<Card.Content class="space-y-2 text-xs text-muted-foreground">
			<p>
				Capture uses the bundled <span class="font-medium text-foreground">WinDivert</span> driver — no
				separate install is required. The game process is identified by its owning PID and ports, so VPNs
				that change addressing are handled automatically for connection tracking.
			</p>
			<p>
				If you use <span class="font-medium text-foreground">ExitLag</span> and see no data, enable
				<span class="font-medium text-foreground">NDIS Legacy mode</span> in ExitLag's settings. If you
				don't need a VPN to play, disabling it gives the most reliable capture.
			</p>
		</Card.Content>
	</Card.Root>
</Tabs.Content>
