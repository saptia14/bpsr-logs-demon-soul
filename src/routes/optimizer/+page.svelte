<script lang="ts">
	import { onMount } from 'svelte';
	import { commands, type AttrMeta, type ModuleInfo, type ModuleSolution } from '$lib/bindings';

	type AttrState = 'target' | 'exclude';

	let attrs = $state<AttrMeta[]>([]);
	let modules = $state<ModuleInfo[]>([]);
	let selection = $state<Record<number, AttrState>>({});

	let category = $state<'All' | 'Attack' | 'Guardian' | 'Support'>('All');
	let comboSize = $state<4 | 5>(4);
	let matchCount = $state(1);

	let solutions = $state<ModuleSolution[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let hasRun = $state(false);

	const basicAttrs = $derived(attrs.filter((a) => !a.special));
	const specialAttrs = $derived(attrs.filter((a) => a.special));
	const targetCount = $derived(Object.values(selection).filter((s) => s === 'target').length);

	async function loadModules() {
		modules = await commands.getModules();
	}
	onMount(async () => {
		attrs = await commands.getModuleAttributes();
		await loadModules();
	});

	function cycleAttr(id: number) {
		const cur = selection[id];
		if (cur === undefined) selection[id] = 'target';
		else if (cur === 'target') selection[id] = 'exclude';
		else delete selection[id];
	}
	function chipState(id: number): string {
		const s = selection[id];
		return s === 'target' ? 'max' : s === 'exclude' ? 'exc' : '';
	}

	async function optimize() {
		loading = true;
		error = null;
		hasRun = true;
		const targetAttrs = Object.entries(selection).filter(([, s]) => s === 'target').map(([id]) => Number(id));
		const excludeAttrs = Object.entries(selection).filter(([, s]) => s === 'exclude').map(([id]) => Number(id));
		try {
			const res = await commands.optimizeModules({ category, targetAttrs, excludeAttrs, matchCount, comboSize, topN: 10 });
			if (res.status === 'ok') solutions = res.data;
			else {
				error = res.error;
				solutions = [];
			}
		} catch (e) {
			error = String(e);
			solutions = [];
		} finally {
			loading = false;
		}
	}
</script>

<div class="screen hud-anim flex h-full flex-col">
	<div class="hud-shead">
		<div class="flex flex-col gap-0.5">
			<h2>Module Optimizer</h2>
			<span class="sub">{modules.length} module{modules.length === 1 ? '' : 's'} parsed</span>
		</div>
		<span class="flex-1"></span>
		<button class="hud-gbtn" onclick={loadModules}>Re-scan</button>
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto px-4 pb-4">
		{#if modules.length === 0}
			<div class="hud-scard" style="border-style:dashed;text-align:center">
				<p style="color:var(--tx-2);font-size:11px">
					No module data yet. Open your character / re-log so the game sends your modules, then press
					<b style="color:var(--ac-bright)">Re-scan</b>.
				</p>
			</div>
		{/if}

		<!-- Controls -->
		<div class="hud-scard flex flex-wrap items-center gap-3">
			<label class="flex items-center gap-2">
				<span class="text-tiny font-semibold uppercase" style="color:var(--tx-2)">Category</span>
				<select bind:value={category} class="hud-sel">
					<option value="All">All</option>
					<option value="Attack">Attack</option>
					<option value="Guardian">Guardian</option>
					<option value="Support">Support</option>
				</select>
			</label>
			<div class="hud-combo flex items-center gap-2">
				<span class="text-tiny font-semibold uppercase" style="color:var(--tx-2)">Combo</span>
				<span class="flex gap-1">
					{#each [4, 5] as size (size)}
						<button class:on={comboSize === size} onclick={() => (comboSize = size as 4 | 5)}>{size}</button>
					{/each}
				</span>
			</div>
			<label class="flex items-center gap-2">
				<span class="text-tiny font-semibold uppercase" style="color:var(--tx-2)">Match ≥</span>
				<input type="number" min="0" max="3" bind:value={matchCount} class="hud-num" />
			</label>
			<button class="hud-gbtn primary ml-auto" onclick={optimize} disabled={loading || modules.length === 0}>
				{loading ? 'Optimizing…' : 'Optimize'}
			</button>
		</div>

		<!-- Attribute picker -->
		<div class="hud-scard">
			<div class="mb-3 flex items-center justify-between" style="font-size:10.5px;color:var(--tx-2)">
				<span>Click to cycle: <b style="color:var(--good)">maximize</b> → <b style="color:var(--bad)">exclude</b> → off</span>
				<span style="font-family:var(--mono)">{targetCount} target{targetCount === 1 ? '' : 's'}</span>
			</div>
			<div class="mb-2 flex items-center gap-2 text-tiny uppercase" style="color:var(--tx-3);letter-spacing:0.16em">Basic</div>
			<div class="flex flex-wrap gap-2">
				{#each basicAttrs as a (a.id)}
					<button class="hud-chip {chipState(a.id)}" onclick={() => cycleAttr(a.id)}><span class="st-dot"></span>{a.name}</button>
				{/each}
			</div>
			<div class="mb-2 mt-4 flex items-center gap-2 text-tiny uppercase" style="color:var(--tx-3);letter-spacing:0.16em">Special (极)</div>
			<div class="flex flex-wrap gap-2">
				{#each specialAttrs as a (a.id)}
					<button class="hud-chip {chipState(a.id)}" onclick={() => cycleAttr(a.id)}><span class="st-dot"></span>{a.name}</button>
				{/each}
			</div>
		</div>

		<!-- Results -->
		{#if error}
			<div class="hud-scard" style="border-color:rgba(251,93,110,0.4);color:var(--bad)">{error}</div>
		{:else if loading}
			<p class="p-4 text-center" style="color:var(--tx-2)">Crunching combinations…</p>
		{:else if hasRun && solutions.length === 0}
			<p class="p-4 text-center" style="color:var(--tx-2)">No combinations matched these filters.</p>
		{/if}

		{#each solutions as sol (sol.rank)}
			<div class="hud-mset" style="flex-direction:column;align-items:stretch;cursor:default">
				<div class="flex items-center gap-3">
					<span class="mrank">#{sol.rank}</span>
					<span class="hud-mtag">Total {sol.totalValue}</span>
					<span class="flex-1"></span>
					<span class="mscore"><span class="v">{sol.score.toFixed(0)}</span> <span class="l">score</span></span>
				</div>
				<ol class="mt-2 list-decimal space-y-1 pl-5" style="font-size:11px;color:var(--tx-1)">
					{#each sol.modules as m (m.uuid)}
						<li>
							<span class="font-semibold" style="color:var(--tx-0)">{m.name}</span>
							<span style="color:var(--tx-3)"> Q{m.quality}</span>
							<span style="color:var(--tx-2)"> — {m.parts.map((p) => `${p.name}+${p.value}`).join(', ')}</span>
						</li>
					{/each}
				</ol>
				<div class="mt-2 flex flex-wrap gap-1.5">
					{#each sol.breakdown as b (b.name)}
						<span class="hud-mtag">{b.name} +{b.value}</span>
					{/each}
				</div>
			</div>
		{/each}
	</div>
</div>
