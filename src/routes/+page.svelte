<script lang="ts">
	import { onMount } from 'svelte';
	import { commands, type PlayersWindow, type PlayerRow } from '$lib/bindings';
	import { getClassColor, getClassIcon, tooltip } from '$lib/utils.svelte';
	import { goto } from '$app/navigation';
	import { SETTINGS } from '$lib/settings-store';
	import AbbreviatedNumber from '$lib/components/abbreviated-number.svelte';

	const GRID = '1fr 70px 66px 42px 44px 52px';

	onMount(() => {
		fetchData();
		const interval = setInterval(fetchData, 200);
		return () => clearInterval(interval);
	});

	let win: PlayersWindow = $state({ playerRows: [], localPlayerUid: -1, topValue: 0 });

	async function fetchData() {
		if (SETTINGS.misc.state.testingMode) {
			win = await commands.getTestPlayerWindow();
		} else if (SETTINGS.general.state.bossOnly) {
			win = await commands.getDpsBossOnlyPlayerWindow();
		} else {
			win = await commands.getDpsPlayerWindow();
		}
	}

	const yourName = $derived(SETTINGS.general.state.showYourName);
	const othersName = $derived(SETTINGS.general.state.showOthersName);

	function isYou(p: PlayerRow): boolean {
		return p.uid !== -1 && p.uid === win.localPlayerUid;
	}
	function classLine(p: PlayerRow): string {
		return p.classSpecName ? p.classSpecName : p.className;
	}
	function displayName(p: PlayerRow): string {
		if (isYou(p)) {
			if (yourName === 'Show Your Class') return classLine(p);
			if (yourName === 'Hide Your Name') return 'Hidden';
			return `${p.name}`;
		}
		if (othersName === "Show Others' Class") return classLine(p);
		if (othersName === "Hide Others' Name") return 'Hidden';
		return p.name;
	}
	function showGs(p: PlayerRow): boolean {
		if (p.abilityScore < 0) return false;
		return isYou(p)
			? SETTINGS.general.state.showYourAbilityScore
			: SETTINGS.general.state.showOthersAbilityScore;
	}
</script>

<div class="hud-meter hud-anim">
	<div class="hud-meter-head" style={`grid-template-columns:${GRID}`}>
		<div class="col lead">Combatant</div>
		<div class="col">DMG</div>
		<div class="col sorted">DPS ↓</div>
		<div class="col">D%</div>
		<div class="col">CR%</div>
		<div class="col">CDMG%</div>
	</div>
	<div class="hud-meter-rows">
		{#each win.playerRows as p, i (p.uid)}
			<div
				class="hud-prow"
				class:you={isYou(p)}
				style={`grid-template-columns:${GRID}; --rc:${getClassColor(p.className)}`}
				onclick={() => goto(`/skills?playerUid=${p.uid}&type=dps`)}
				role="button"
				tabindex="0"
				onkeydown={(e) => e.key === 'Enter' && goto(`/skills?playerUid=${p.uid}&type=dps`)}
			>
				<div
					class="fill"
					style={`width:${Math.max(2, (p.totalValue / (win.topValue || 1)) * 100)}%`}
				></div>
				<div class="hud-pl-lead">
					<span class="hud-prank">{i + 1}</span>
					<span class="hud-emblem">
						<img src={getClassIcon(p.className)} alt={p.className} />
					</span>
					<span class="hud-pname">
						<span class="nm" {@attach tooltip(() => `#${p.uid}`)}>{displayName(p)}</span>
						<span class="cls">{classLine(p)}</span>
					</span>
					{#if showGs(p)}
						<span class="hud-pgs">{Math.round(p.abilityScore)}</span>
					{/if}
				</div>
				<div class="hud-pm"><span class="big"><AbbreviatedNumber num={p.totalValue} /></span></div>
				<div class="hud-pm accent"><span class="big"><AbbreviatedNumber num={p.valuePerSec} /></span></div>
				<div class="hud-pm"><span class="pct" class:zero={p.valuePct === 0}>{p.valuePct.toFixed(0)}%</span></div>
				<div class="hud-pm"><span class="pct dim">{p.critRate.toFixed(0)}%</span></div>
				<div class="hud-pm"><span class="pct dim">{p.critValueRate.toFixed(0)}%</span></div>
			</div>
		{/each}
		{#if win.playerRows.length === 0}
			<div class="hud-empty">
				<div class="ico">
					<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12h4l2.5 7 5-14L17 12h4"/></svg>
				</div>
				<p>Waiting for combat…</p>
			</div>
		{/if}
	</div>
</div>
