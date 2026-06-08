<script lang="ts">
	import { onMount } from 'svelte';
	import { commands, type SkillsWindow } from '$lib/bindings';
	import { getClassColor, getClassIcon, getSkillIcon } from '$lib/utils.svelte';
	import { page } from '$app/state';
	import { SETTINGS } from '$lib/settings-store';
	import AbbreviatedNumber from '$lib/components/abbreviated-number.svelte';

	const SKGRID = '26px 1fr 54px 50px 42px 44px';

	const playerUidParam = page.url.searchParams.get('playerUid');
	const playerUid: string = playerUidParam && /^-?\d+$/.test(playerUidParam) ? playerUidParam : '-1';
	const typeParam = page.url.searchParams.get('type');
	const statType: 'dps' | 'heal' = typeParam === 'heal' ? 'heal' : 'dps';

	onMount(() => {
		fetchData();
		const interval = setInterval(fetchData, 200);
		return () => clearInterval(interval);
	});

	let win = $state<SkillsWindow | undefined>(undefined);

	async function fetchData() {
		try {
			const result = SETTINGS.misc.state.testingMode
				? await commands.getTestSkillWindow(playerUid)
				: statType === 'dps'
					? await commands.getDpsSkillWindow(playerUid)
					: await commands.getHealSkillWindow(playerUid);
			if (result.status === 'ok') win = result.data;
		} catch (e) {
			console.error('Error fetching data: ', e);
		}
	}

	const topSkill = $derived(
		win ? Math.max(1, ...win.skillRows.map((s) => s.totalValue)) : 1
	);
</script>

<svelte:window oncontextmenu={() => window.history.back()} />

{#if win}
	{@const ip = win.inspectedPlayer}
	{@const rc = getClassColor(ip.className)}
	<div class="screen hud-anim flex h-full flex-col">
		<div class="hud-shead" style="padding-bottom:0">
			<button class="hud-gbtn" onclick={() => window.history.back()}>← Back</button>
			<span class="flex-1"></span>
			<span class="sub">{statType === 'heal' ? 'Healing' : 'Damage'} breakdown</span>
		</div>

		<div class="hud-pdetail-hero" style={`--rc:${rc}; margin-top:11px`}>
			<span class="hud-emblem"><img src={getClassIcon(ip.className)} alt={ip.className} /></span>
			<div class="hud-pdetail-id">
				<div class="nm">{ip.name}</div>
				<div class="meta">
					<span class="cls">{ip.classSpecName || ip.className}</span>
					{#if ip.abilityScore >= 0}<span class="gs">GS {Math.round(ip.abilityScore)}</span>{/if}
				</div>
			</div>
			<div class="hud-pdetail-stats">
				<div class="st">
					<div class="k">{statType === 'heal' ? 'Total Heal' : 'Total DMG'}</div>
					<div class="v"><AbbreviatedNumber num={ip.totalValue} /></div>
				</div>
				<div class="st">
					<div class="k">{statType === 'heal' ? 'HPS' : 'DPS'}</div>
					<div class="v ac"><AbbreviatedNumber num={ip.valuePerSec} /></div>
				</div>
			</div>
		</div>

		<div class="min-h-0 flex-1 overflow-y-auto">
			{#each win.skillRows as s (s.uid)}
				<div class="hud-skrow" style={`grid-template-columns:${SKGRID}`}>
					<div class="fill" style={`width:${(s.totalValue / topSkill) * 100}%`}></div>
					<span class="hud-skicon"><img src={getSkillIcon(s.uid)} alt={s.name} /></span>
					<div class="hud-sk-id">
						<div class="nm">{s.name}</div>
						<div class="sub">{s.hits} hits</div>
					</div>
					<div class="hud-sk-m"><div class="v"><AbbreviatedNumber num={s.totalValue} /></div><div class="l">total</div></div>
					<div class="hud-sk-m"><div class="v"><AbbreviatedNumber num={s.valuePerSec} /></div><div class="l">/sec</div></div>
					<div class="hud-sk-m"><div class="v" style="color:var(--ac-bright)">{s.critRate.toFixed(0)}%</div><div class="l">crit</div></div>
					<div class="hud-sk-m"><div class="v">{s.valuePct.toFixed(0)}%</div><div class="l">share</div></div>
				</div>
			{/each}
		</div>
	</div>
{/if}
