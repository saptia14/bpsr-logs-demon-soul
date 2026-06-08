<script lang="ts">
	import { onMount } from 'svelte';
	import { commands, type EncounterSummary, type EncounterDetail } from '$lib/bindings';
	import { getClassColor, getClassIcon } from '$lib/utils.svelte';
	import AbbreviatedNumber from '$lib/components/abbreviated-number.svelte';
	import TrashIcon from 'virtual:icons/lucide/trash-2';
	import RefreshIcon from 'virtual:icons/lucide/refresh-cw';

	let history = $state<EncounterSummary[]>([]);
	let detail = $state<EncounterDetail | null>(null);
	let loading = $state(true);
	let error = $state('');
	let expandedUid = $state<number | null>(null);

	onMount(loadHistory);

	async function loadHistory() {
		loading = true;
		error = '';
		const res = await commands.getEncounterHistory(200);
		if (res.status === 'ok') history = res.data;
		else error = res.error;
		loading = false;
	}
	async function openDetail(id: number) {
		expandedUid = null;
		const res = await commands.getEncounterDetail(id);
		if (res.status === 'ok') detail = res.data;
		else error = res.error;
	}
	async function removeOne(id: number, e: MouseEvent) {
		e.stopPropagation();
		await commands.deleteEncounter(id);
		history = history.filter((h) => h.id !== id);
	}
	async function clearAll() {
		if (!history.length) return;
		await commands.clearEncounterHistory();
		history = [];
		detail = null;
	}
	function fmtDate(ms: number): string {
		return new Date(ms).toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
	}
	function fmtDuration(ms: number): string {
		const total = Math.floor(ms / 1000);
		return `${String(Math.floor(total / 60)).padStart(2, '0')}:${String(total % 60).padStart(2, '0')}`;
	}
	function maxOr1(nums: number[]): number {
		return Math.max(1, ...nums);
	}
	const detailTop = $derived(detail ? maxOr1(detail.players.map((p) => p.totalDamage)) : 1);
</script>

<div class="flex h-full flex-col hud-anim">
	{#if detail}
		<div class="hud-shead">
			<button class="hud-gbtn" onclick={() => (detail = null)}>← Back</button>
			<span class="flex-1"></span>
			<h2 class="truncate">{detail.summary.mapName}</h2>
			<span class="sub">{fmtDuration(detail.summary.durationMs)}</span>
		</div>
		<div class="min-h-0 flex-1 overflow-y-auto">
			{#each detail.players as p (p.uid)}
				{@const rc = getClassColor(p.className)}
				<div
					class="hud-prow"
					style={`grid-template-columns:1fr 64px 76px; --rc:${rc}`}
					role="button"
					tabindex="0"
					onclick={() => (expandedUid = expandedUid === p.uid ? null : p.uid)}
					onkeydown={(e) => e.key === 'Enter' && (expandedUid = expandedUid === p.uid ? null : p.uid)}
				>
					<div class="fill" style={`width:${(p.totalDamage / detailTop) * 100}%`}></div>
					<div class="hud-pl-lead">
						<span class="hud-emblem"><img src={getClassIcon(p.className)} alt={p.className} /></span>
						<span class="hud-pname">
							<span class="nm">{p.name}</span>
							<span class="cls">{p.classSpecName || p.className}</span>
						</span>
					</div>
					<div class="hud-pm"><span class="pct dim">{p.hits > 0 ? Math.round((p.critHits / p.hits) * 100) : 0}% cr</span></div>
					<div class="hud-pm accent"><span class="big"><AbbreviatedNumber num={p.totalDamage} /></span></div>
				</div>
				{#if expandedUid === p.uid}
					{@const skTop = maxOr1(p.skills.map((s) => s.totalDamage + s.totalHealing))}
					{#each p.skills as s (s.uid)}
						<div class="hud-skrow" style="grid-template-columns:1fr 56px 56px; background:rgba(255,255,255,0.015)">
							<div class="fill" style={`width:${((s.totalDamage + s.totalHealing) / skTop) * 100}%; background:linear-gradient(90deg, color-mix(in oklab, ${rc} 18%, transparent), transparent); border-color:${rc}`}></div>
							<div class="hud-sk-id"><div class="nm">{s.name}</div><div class="sub">{s.hits} hits</div></div>
							<div class="hud-sk-m"><div class="v">{s.hits}</div><div class="l">hits</div></div>
							<div class="hud-sk-m"><div class="v"><AbbreviatedNumber num={s.totalDamage + s.totalHealing} /></div><div class="l">total</div></div>
						</div>
					{/each}
				{/if}
			{/each}
		</div>
	{:else}
		<div class="hud-shead">
			<h2>Encounter History</h2>
			<span class="flex-1"></span>
			<button class="hud-gbtn" onclick={loadHistory} aria-label="Refresh"><RefreshIcon class="size-3.5" /></button>
			<button class="hud-gbtn danger" onclick={clearAll}><TrashIcon class="size-3.5" /> Clear</button>
		</div>
		<div class="hud-hist-body min-h-0 flex-1 overflow-y-auto px-3 pb-3">
			{#if error}
				<p class="p-4 text-center" style="color:var(--bad)">{error}</p>
			{:else if loading}
				<p class="p-4 text-center" style="color:var(--tx-2)">Loading…</p>
			{:else if history.length === 0}
				<div class="hud-empty"><p>No saved encounters yet. Every fight is stored automatically.</p></div>
			{:else}
				{#each history as h (h.id)}
					<div
						class="hud-enc"
						role="button"
						tabindex="0"
						onclick={() => openDetail(h.id)}
						onkeydown={(e) => e.key === 'Enter' && openDetail(h.id)}
					>
						<div class="hud-enc-time">
							<div class="t">{fmtDuration(h.durationMs)}</div>
							<div class="d">{fmtDate(h.createdAt)}</div>
						</div>
						<div class="hud-enc-id">
							<div class="nm"><span class="zone" style="width:6px;height:6px;border-radius:2px;background:var(--ac);flex-shrink:0"></span>{h.mapName}</div>
							<div class="meta"><b>{h.playerCount}</b> players · top <b>{h.topPlayerName}</b></div>
						</div>
						<div class="hud-enc-score">
							<div class="v"><AbbreviatedNumber num={h.totalDps} /><em>/s</em></div>
							<div class="tot"><AbbreviatedNumber num={h.totalDamage} /></div>
						</div>
						<button class="hud-enc-del" onclick={(e) => removeOne(h.id, e)} aria-label="Delete encounter">
							<TrashIcon class="size-3.5" />
						</button>
					</div>
				{/each}
			{/if}
		</div>
	{/if}
</div>
