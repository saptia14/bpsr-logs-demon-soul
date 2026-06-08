<script lang="ts">
	import { onMount } from 'svelte';
	import {
		commands,
		type EncounterSummary,
		type EncounterDetail,
		type StoredPlayer
	} from '$lib/bindings';
	import { getClassColor, getClassIcon } from '$lib/utils.svelte';
	import { SETTINGS } from '$lib/settings-store';
	import AbbreviatedNumber from '$lib/components/abbreviated-number.svelte';
	import TrashIcon from 'virtual:icons/lucide/trash-2';
	import ChevronLeftIcon from 'virtual:icons/lucide/chevron-left';
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
		if (res.status === 'ok') {
			history = res.data;
		} else {
			error = res.error;
		}
		loading = false;
	}

	async function openDetail(id: number) {
		expandedUid = null;
		const res = await commands.getEncounterDetail(id);
		if (res.status === 'ok') {
			detail = res.data;
		} else {
			error = res.error;
		}
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
		const d = new Date(ms);
		return d.toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function fmtDuration(ms: number): string {
		const total = Math.floor(ms / 1000);
		const m = Math.floor(total / 60);
		const s = total % 60;
		return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
	}

	function critRate(p: StoredPlayer): string {
		if (p.hits <= 0) return '0%';
		return `${Math.round((p.critHits / p.hits) * 100)}%`;
	}

	function maxOr1(nums: number[]): number {
		return Math.max(1, ...nums);
	}
	const detailTop = $derived(detail ? maxOr1(detail.players.map((p) => p.totalDamage)) : 1);
	const opacity = $derived(SETTINGS.accessibility.state.transparencyOpacity / 100);
</script>

<div class="flex h-full flex-col text-xs">
	<!-- Toolbar -->
	<div
		class="sticky top-0 z-10 flex items-center justify-between gap-2 border-b px-2 py-1"
		style={`background-color: oklch(from var(--card) l c h / ${opacity});`}
	>
		{#if detail}
			<button
				class="flex items-center gap-1 rounded-md px-1.5 py-0.5 font-medium hover:bg-accent"
				onclick={() => (detail = null)}
			>
				<ChevronLeftIcon class="size-3.5" /> Back
			</button>
			<span class="truncate font-semibold">{detail.summary.mapName}</span>
			<span class="text-muted-foreground">{fmtDuration(detail.summary.durationMs)}</span>
		{:else}
			<span class="font-semibold">Encounter History</span>
			<span class="flex items-center gap-1">
				<button
					class="flex items-center gap-1 rounded-md px-1.5 py-0.5 hover:bg-accent"
					onclick={loadHistory}
					aria-label="Refresh"
				>
					<RefreshIcon class="size-3.5" />
				</button>
				<button
					class="flex items-center gap-1 rounded-md px-1.5 py-0.5 text-destructive hover:bg-destructive/15"
					onclick={clearAll}
					aria-label="Clear all history"
				>
					<TrashIcon class="size-3.5" /> Clear
				</button>
			</span>
		{/if}
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto">
		{#if error}
			<p class="p-3 text-center text-destructive">{error}</p>
		{:else if loading}
			<p class="p-3 text-center text-muted-foreground">Loading…</p>
		{:else if detail}
			<!-- Detail: per-player breakdown -->
			<table class="w-full table-fixed">
				<tbody>
					{#each detail.players as p (p.uid)}
						<tr
							class="relative h-7 cursor-pointer overflow-hidden border-b hover:bg-accent/40"
							onclick={() => (expandedUid = expandedUid === p.uid ? null : p.uid)}
						>
							<td class="relative z-10 w-7 pl-1">
								<img src={getClassIcon(p.className)} alt={p.className} class="size-5" />
							</td>
							<td class="relative z-10 truncate text-left">
								<span class="font-medium">{p.name}</span>
								<span class="text-muted-foreground">{p.classSpecName || p.className}</span>
							</td>
							<td class="relative z-10 w-16 text-right text-muted-foreground">{critRate(p)} cr</td>
							<td class="relative z-10 w-20 pr-2 text-right font-semibold">
								<AbbreviatedNumber num={p.totalDamage} />
							</td>
							<td
								class="pointer-events-none absolute top-0 left-0 h-7"
								style="background-color: {getClassColor(p.className)}; width: {(p.totalDamage /
									detailTop) *
									100}%; opacity: {Math.max(0.3, opacity)};"
							></td>
						</tr>
						{#if expandedUid === p.uid}
							{@const skillTop = Math.max(
								1,
								...p.skills.map((s) => s.totalDamage + s.totalHealing)
							)}
							{#each p.skills as s (s.uid)}
								<tr class="relative h-6 overflow-hidden bg-muted/30">
									<td></td>
									<td class="relative z-10 truncate pl-1 text-left text-muted-foreground"
										>{s.name}</td
									>
									<td class="relative z-10 text-right text-muted-foreground">{s.hits} hits</td>
									<td class="relative z-10 w-20 pr-2 text-right">
										<AbbreviatedNumber num={s.totalDamage + s.totalHealing} />
									</td>
									<td
										class="pointer-events-none absolute top-0 left-0 h-6 bg-foreground/10"
										style="width: {((s.totalDamage + s.totalHealing) / skillTop) * 100}%;"
									></td>
								</tr>
							{/each}
						{/if}
					{/each}
				</tbody>
			</table>
		{:else if history.length === 0}
			<p class="p-4 text-center text-muted-foreground">
				No saved encounters yet. Fight something — every encounter is stored automatically.
			</p>
		{:else}
			<!-- Master: list of encounters -->
			{#each history as h (h.id)}
				<div
					class="flex w-full cursor-pointer items-center gap-2 border-b px-2 py-1.5 text-left hover:bg-accent/40"
					role="button"
					tabindex="0"
					onclick={() => openDetail(h.id)}
					onkeydown={(e) => e.key === 'Enter' && openDetail(h.id)}
				>
					<span class="flex w-12 shrink-0 flex-col items-start">
						<span class="font-mono text-muted-foreground">{fmtDuration(h.durationMs)}</span>
						<span class="text-tiny text-muted-foreground">{fmtDate(h.createdAt)}</span>
					</span>
					<span class="flex min-w-0 flex-1 flex-col">
						<span class="truncate font-medium">{h.mapName}</span>
						<span class="truncate text-tiny text-muted-foreground">
							{h.playerCount} players · top {h.topPlayerName}
						</span>
					</span>
					<span class="flex shrink-0 flex-col items-end">
						<span class="font-semibold"><AbbreviatedNumber num={h.totalDps} />/s</span>
						<span class="text-tiny text-muted-foreground"
							><AbbreviatedNumber num={h.totalDamage} /></span
						>
					</span>
					<button
						class="shrink-0 rounded p-1 text-muted-foreground hover:bg-destructive/15 hover:text-destructive"
						onclick={(e) => removeOne(h.id, e)}
						aria-label="Delete encounter"
					>
						<TrashIcon class="size-3.5" />
					</button>
				</div>
			{/each}
		{/if}
	</div>
</div>
