/* ============================================================
   BPSR Logs — meter screens: DPS, Heal, Skill breakdown
   ============================================================ */
const { useState: useStateM } = React;

/* ---------- shared player row ---------- */
function PlayerRow({ p, rank, topVal, valKey, cols, density, onOpen }) {
	const info = B.CLASSES[p.cls] || { color: 'var(--tx-2)' };
	const pct = Math.max(2, (p[valKey] / topVal) * 100);
	return (
		<div className={`prow ${density} ${p.you ? 'you' : ''}`}
			style={{ '--rc': info.color, gridTemplateColumns: cols.grid }}
			onClick={() => onOpen && onOpen(p)}>
			<div className="fill" style={{ width: pct + '%' }}></div>
			<div className="pl-lead">
				<span className="prank">{rank}</span>
				<Emblem cls={p.cls} />
				<div className="pname">
					<span className="nm">{p.name}</span>
					{density === 'expanded' && <span className="cls">{B.CLASSES[p.cls]?.short || p.cls}</span>}
				</div>
				{density === 'expanded' && <span className="pgs">{p.gs}</span>}
			</div>
			{cols.metrics.map((m, i) => <MetricCell key={i} p={p} m={m} />)}
		</div>
	);
}

function MetricCell({ p, m }) {
	if (m.type === 'num') return <Num n={p[m.key]} cls={`pm ${m.accent ? 'accent' : ''}`} />;
	if (m.type === 'pct') {
		const v = p[m.key];
		return <div className="pm"><span className={`pct ${v === 0 ? 'zero' : m.dim ? 'dim' : ''}`}>{v}%</span></div>;
	}
	return null;
}

/* ---------- DPS screen ---------- */
function DpsScreen({ density, onOpen }) {
	const players = B.DPS_PLAYERS;
	const topVal = players[0].dmg;
	const cols = density === 'compact'
		? { grid: '1fr 74px 50px', metrics: [{ type: 'num', key: 'dps', accent: true }, { type: 'pct', key: 'dPct' }],
		    head: [{ l: 'DPS', sorted: true }, { l: 'D%' }] }
		: { grid: '1fr 70px 66px 42px 44px 52px',
		    metrics: [{ type: 'num', key: 'dmg' }, { type: 'num', key: 'dps', accent: true }, { type: 'pct', key: 'dPct' }, { type: 'pct', key: 'cr', dim: true }, { type: 'pct', key: 'cdmg', dim: true }],
		    head: [{ l: 'DMG' }, { l: 'DPS', sorted: true }, { l: 'D%' }, { l: 'CR%' }, { l: 'CDMG%' }] };
	return (
		<div className="meter screen-anim">
			<MeterHead cols={cols} />
			<div className="meter-rows">
				{players.map((p, i) => <PlayerRow key={p.uid} p={p} rank={i + 1} topVal={topVal} valKey="dmg" cols={cols} density={density} onOpen={onOpen} />)}
			</div>
		</div>
	);
}

/* ---------- Heal screen ---------- */
function HealScreen({ density, onOpen }) {
	const players = B.HEAL_PLAYERS;
	const topVal = players[0].heal;
	const cols = density === 'compact'
		? { grid: '1fr 74px 50px', metrics: [{ type: 'num', key: 'hps', accent: true }, { type: 'pct', key: 'hPct' }],
		    head: [{ l: 'HPS', sorted: true }, { l: 'H%' }] }
		: { grid: '1fr 70px 66px 42px 50px 44px',
		    metrics: [{ type: 'num', key: 'heal' }, { type: 'num', key: 'hps', accent: true }, { type: 'pct', key: 'hPct' }, { type: 'pct', key: 'over', dim: true }, { type: 'pct', key: 'cr', dim: true }],
		    head: [{ l: 'HEAL' }, { l: 'HPS', sorted: true }, { l: 'H%' }, { l: 'OVR%' }, { l: 'CR%' }] };
	return (
		<div className="meter screen-anim">
			<MeterHead cols={cols} />
			<div className="meter-rows">
				{players.map((p, i) => <PlayerRow key={p.uid} p={p} rank={i + 1} topVal={topVal} valKey="heal" cols={cols} density={density} onOpen={onOpen} />)}
			</div>
		</div>
	);
}

function MeterHead({ cols }) {
	return (
		<div className="meter-head" style={{ gridTemplateColumns: cols.grid }}>
			<div className="col lead">Combatant</div>
			{cols.head.map((h, i) => (
				<div key={i} className={`col sortable ${h.sorted ? 'sorted' : ''}`}>{h.l}{h.sorted ? ' ↓' : ''}</div>
			))}
		</div>
	);
}

/* ---------- Skill breakdown ---------- */
function SkillScreen({ player, type, onBack }) {
	const data = (B.SKILLS[player.uid] && B.SKILLS[player.uid].skills) || B.GENERIC_SKILLS;
	const top = Math.max(...data.map(s => s.total));
	const info = B.CLASSES[player.cls] || { color: 'var(--tx-2)' };
	const totalVal = type === 'heal' ? player.heal : player.dmg;
	const perSec = type === 'heal' ? player.hps : player.dps;
	return (
		<div className="screen screen-anim">
			<div className="shead" style={{ paddingBottom: 0 }}>
				<button className="gbtn" onClick={onBack}><Icon name="back" size={13} /> Back</button>
				<div className="shead-spacer"></div>
				<span className="shead-title"><span className="sub">{type === 'heal' ? 'Healing' : 'Damage'} breakdown</span></span>
			</div>
			<div className="pdetail-hero" style={{ '--rc': info.color, marginTop: 11 }}>
				<Emblem cls={player.cls} style={{ width: 40, height: 40, borderRadius: 10 }} />
				<div className="pdetail-id">
					<div className="nm">{player.name}</div>
					<div className="meta">
						<span className="cls">{B.CLASSES[player.cls]?.short || player.cls}</span>
						<span className="gs">GS {player.gs}</span>
					</div>
				</div>
				<div className="pdetail-stats">
					<div className="st"><div className="k">{type === 'heal' ? 'Total Heal' : 'Total DMG'}</div><div className="v">{(() => { const f = fmt(totalVal); return f.v + f.u; })()}</div></div>
					<div className="st"><div className="k">{type === 'heal' ? 'HPS' : 'DPS'}</div><div className="v ac">{(() => { const f = fmt(perSec); return f.v + f.u; })()}</div></div>
				</div>
			</div>
			<div className="scroll">
				{data.map((s, i) => {
					const f = fmt(s.total), fd = fmt(s.dps);
					return (
						<div className="skrow" key={i}>
							<div className="fill" style={{ width: (s.total / top * 100) + '%' }}></div>
							<div className="skicon"><Icon name={i % 3 === 0 ? 'zap' : i % 3 === 1 ? 'spark' : 'sword'} size={14} fill={i % 3 === 0} /></div>
							<div className="sk-id"><div className="nm">{s.name}</div><div className="sub">{s.sub} · {s.hits} hits</div></div>
							<div className="sk-m"><div className="v">{f.v}<em>{f.u}</em></div><div className="l">total</div></div>
							<div className="sk-m"><div className="v">{fd.v}<em>{fd.u}</em></div><div className="l">/sec</div></div>
							<div className="sk-m"><div className="v" style={{ color: 'var(--ac-bright)' }}>{s.cr}%</div><div className="l">crit</div></div>
							<div className="sk-m"><div className="v">{s.pct}%</div><div className="l">share</div></div>
						</div>
					);
				})}
			</div>
		</div>
	);
}

Object.assign(window, { DpsScreen, HealScreen, SkillScreen });
