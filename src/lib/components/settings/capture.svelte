<script lang="ts">
	import * as Tabs from '$lib/components/ui/tabs/index.js';
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

<Tabs.Content value={SETTINGS_CATEGORY}>
	<div class="hud-scard">
		<h3><span style="color:var(--ac)">◎</span> Connection Status</h3>
		<div class="hud-srow">
			<div class="si flex items-center gap-2">
				<span
					class="inline-block"
					style={`width:8px;height:8px;border-radius:99px;background:${diag.gameDetected ? 'var(--good)' : 'var(--bad)'};box-shadow:0 0 8px ${diag.gameDetected ? 'var(--good)' : 'var(--bad)'}`}
				></span>
				<span class="lab">{diag.gameDetected ? 'Game process detected' : 'Game process not detected'}</span>
				{#if diag.processCount > 1}
					<span style="color:var(--tx-2);font-size:11px">({diag.processCount} clients)</span>
				{/if}
			</div>
		</div>
		<div class="hud-srow" style="flex-direction:column;align-items:stretch">
			<div class="si">
				<div class="des">Tracked game TCP ports</div>
				{#if diag.ports.length > 0}
					<div class="mt-2 flex flex-wrap gap-1.5">
						{#each diag.ports as port (port)}
							<span class="hud-mtag" style="background:var(--bg-3);color:var(--tx-1)">{port}</span>
						{/each}
					</div>
				{:else}
					<p class="mt-1.5 text-tiny" style="color:var(--tx-3)">None yet — open the game and load into a zone.</p>
				{/if}
			</div>
		</div>
		<div class="hud-srow">
			<div class="si">
				<div class="lab">Restart Packet Capture</div>
				<div class="des">Reopen the capture handle. Use this if no data appears or after changing your VPN/network setup.</div>
			</div>
			<button class="hud-gbtn" onclick={() => commands.hardReset()}>Restart</button>
		</div>
	</div>

	<div class="hud-scard">
		<h3><span style="color:var(--ac)">⚡</span> VPN &amp; ExitLag</h3>
		<p class="set-sub" style="font-size:10.5px;color:var(--tx-2);line-height:1.5">
			Capture uses the bundled <b style="color:var(--tx-1)">WinDivert</b> driver — no separate install needed.
			The game process is identified by its owning PID and ports, so VPNs that change addressing are handled
			automatically for connection tracking.
		</p>
		<p class="set-sub mt-2" style="font-size:10.5px;color:var(--tx-2);line-height:1.5">
			If you use <b style="color:var(--tx-1)">ExitLag</b> and see no data, enable
			<b style="color:var(--tx-1)">NDIS Legacy mode</b> in ExitLag's settings. If you don't need a VPN to play,
			disabling it gives the most reliable capture.
		</p>
	</div>
</Tabs.Content>
