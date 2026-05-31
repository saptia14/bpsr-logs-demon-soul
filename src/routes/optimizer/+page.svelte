<script lang="ts">
	import { onMount } from 'svelte';
	import { commands, type AttrMeta, type ModuleInfo, type ModuleSolution } from '$lib/bindings';
	import { cn } from '$lib/utils';

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

	// Click cycles an attribute: maximize → exclude → off.
	function cycleAttr(id: number) {
		const cur = selection[id];
		if (cur === undefined) selection[id] = 'target';
		else if (cur === 'target') selection[id] = 'exclude';
		else delete selection[id];
	}

	async function optimize() {
		loading = true;
		error = null;
		hasRun = true;
		const targetAttrs = Object.entries(selection)
			.filter(([, s]) => s === 'target')
			.map(([id]) => Number(id));
		const excludeAttrs = Object.entries(selection)
			.filter(([, s]) => s === 'exclude')
			.map(([id]) => Number(id));

		try {
			const res = await commands.optimizeModules({
				category,
				targetAttrs,
				excludeAttrs,
				matchCount,
				comboSize,
				topN: 10
			});
			if (res.status === 'ok') {
				solutions = res.data;
			} else {
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

	function chipClass(id: number): string {
		const s = selection[id];
		if (s === 'target') return 'bg-primary text-primary-foreground border-primary';
		if (s === 'exclude')
			return 'bg-destructive/80 text-white border-destructive line-through';
		return 'bg-transparent border-border hover:bg-accent hover:text-accent-foreground';
	}
</script>

<div class="flex h-full flex-col gap-3 overflow-y-auto p-3 text-xs">
	<!-- Header -->
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-sm font-semibold">Module Optimizer</h1>
			<p class="text-muted-foreground">
				{modules.length} module{modules.length === 1 ? '' : 's'} parsed
			</p>
		</div>
		<button
			class="rounded-md border px-2 py-1 font-medium transition-colors hover:bg-accent hover:text-accent-foreground"
			onclick={loadModules}
		>
			Re-scan
		</button>
	</div>

	{#if modules.length === 0}
		<div class="rounded-md border border-dashed p-4 text-center text-muted-foreground">
			No module data yet. Open your character / re-log in so the game sends your module data,
			then press <span class="font-medium">Re-scan</span>.
		</div>
	{/if}

	<!-- Controls -->
	<div class="flex flex-wrap items-center gap-3 rounded-md border p-2">
		<label class="flex items-center gap-1.5">
			<span class="text-muted-foreground">Category</span>
			<select
				bind:value={category}
				class="rounded-md border bg-background px-1.5 py-0.5"
			>
				<option value="All">All</option>
				<option value="Attack">Attack</option>
				<option value="Guardian">Guardian</option>
				<option value="Support">Support</option>
			</select>
		</label>

		<div class="flex items-center gap-1.5">
			<span class="text-muted-foreground">Combo</span>
			{#each [4, 5] as size (size)}
				<button
					class={cn(
						'rounded-md border px-2 py-0.5 font-medium transition-colors',
						comboSize === size
							? 'bg-primary text-primary-foreground border-primary'
							: 'hover:bg-accent hover:text-accent-foreground'
					)}
					onclick={() => (comboSize = size as 4 | 5)}
				>
					{size}
				</button>
			{/each}
		</div>

		<label class="flex items-center gap-1.5">
			<span class="text-muted-foreground">Match ≥</span>
			<input
				type="number"
				min="0"
				max="3"
				bind:value={matchCount}
				class="w-12 rounded-md border bg-background px-1.5 py-0.5"
			/>
		</label>

		<button
			class="ml-auto rounded-md bg-primary px-3 py-1 font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-50"
			onclick={optimize}
			disabled={loading || modules.length === 0}
		>
			{loading ? 'Optimizing…' : 'Optimize'}
		</button>
	</div>

	<!-- Attribute picker -->
	<div class="rounded-md border p-2">
		<div class="mb-1.5 flex items-center justify-between text-muted-foreground">
			<span>Attributes — click to cycle: <span class="text-primary">maximize</span> →
				<span class="text-destructive">exclude</span> → off</span>
			<span>{targetCount} target{targetCount === 1 ? '' : 's'}</span>
		</div>
		<div class="mb-1 text-[0.65rem] uppercase tracking-wide text-muted-foreground">Basic</div>
		<div class="flex flex-wrap gap-1">
			{#each basicAttrs as a (a.id)}
				<button class={cn('rounded-full border px-2 py-0.5 transition-colors', chipClass(a.id))} onclick={() => cycleAttr(a.id)}>
					{a.name}
				</button>
			{/each}
		</div>
		<div class="mb-1 mt-2 text-[0.65rem] uppercase tracking-wide text-muted-foreground">Special (极)</div>
		<div class="flex flex-wrap gap-1">
			{#each specialAttrs as a (a.id)}
				<button class={cn('rounded-full border px-2 py-0.5 transition-colors', chipClass(a.id))} onclick={() => cycleAttr(a.id)}>
					{a.name}
				</button>
			{/each}
		</div>
	</div>

	<!-- Results -->
	{#if error}
		<div class="rounded-md border border-destructive/50 bg-destructive/10 p-2 text-destructive">
			{error}
		</div>
	{/if}

	{#if loading}
		<div class="p-4 text-center text-muted-foreground">Crunching combinations…</div>
	{:else if hasRun && !error && solutions.length === 0}
		<div class="p-4 text-center text-muted-foreground">
			No combinations matched these filters.
		</div>
	{/if}

	<div class="flex flex-col gap-2">
		{#each solutions as sol (sol.rank)}
			<div class="rounded-md border p-2">
				<div class="mb-1 flex items-center justify-between font-medium">
					<span>Rank #{sol.rank}</span>
					<span class="text-primary">Score {sol.score.toFixed(2)}</span>
					<span class="text-muted-foreground">Total {sol.totalValue}</span>
				</div>
				<ol class="mb-1.5 list-decimal space-y-0.5 pl-5">
					{#each sol.modules as m (m.uuid)}
						<li>
							<span class="font-medium">{m.name}</span>
							<span class="text-muted-foreground">(Q{m.quality})</span>
							<span class="text-muted-foreground">
								— {m.parts.map((p) => `${p.name}+${p.value}`).join(', ')}
							</span>
						</li>
					{/each}
				</ol>
				<div class="flex flex-wrap gap-1">
					{#each sol.breakdown as b (b.name)}
						<span class="rounded-full bg-accent px-2 py-0.5 text-accent-foreground">
							{b.name} +{b.value}
						</span>
					{/each}
				</div>
			</div>
		{/each}
	</div>
</div>
