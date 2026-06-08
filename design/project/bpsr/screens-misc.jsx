/* ============================================================
   BPSR Logs — Modules · History · Chat · Settings
   ============================================================ */

/* ---------- Module optimizer ---------- */
function ModulesScreen() {
	const [combo, setCombo] = React.useState(4);
	const [category, setCategory] = React.useState('All');
	const [match, setMatch] = React.useState(1);
	const [states, setStates] = React.useState({}); // attr -> 'max' | 'exc'
	const [results, setResults] = React.useState(null);

	function cycle(a) {
		setStates(s => {
			const cur = s[a];
			const next = cur === 'max' ? 'exc' : cur === 'exc' ? undefined : 'max';
			const out = { ...s };
			if (next) out[a] = next; else delete out[a];
			return out;
		});
		setResults(null);
	}
	const targets = Object.values(states).filter(v => v === 'max').length;

	function optimize() {
		const picks = Object.keys(states).filter(a => states[a] === 'max');
		const base = picks.length ? picks : ['Crit Focus', 'Special Attack'];
		setResults([
			{ rank: 1, slots: base.slice(0, combo), score: 4280, tag: 'BEST' },
			{ rank: 2, slots: base.slice(0, combo), score: 4115, tag: null },
			{ rank: 3, slots: base.slice(0, combo), score: 3990, tag: null }
		]);
	}

	function chipClass(a) { return `chip ${states[a] === 'max' ? 'max' : states[a] === 'exc' ? 'exc' : ''}`; }

	return (
		<div className="screen screen-anim">
			<div className="shead">
				<span className="shead-title">
					<h2><Icon name="grid" size={15} style={{ color: 'var(--ac)' }} /> Module Optimizer</h2>
					<span className="sub">1307 modules parsed</span>
				</span>
				<div className="shead-spacer"></div>
				<button className="gbtn"><Icon name="refresh" size={13} /> Re-scan</button>
			</div>

			<div className="opt-body">
				<div className="opt-controls">
					<div className="field">
						<label>Category</label>
						<select className="sel" value={category} onChange={e => setCategory(e.target.value)}>
							<option>All</option><option>Offense</option><option>Defense</option><option>Support</option>
						</select>
					</div>
					<div className="field">
						<label>Combo</label>
						<div className="combo-pick">
							<button className={combo === 4 ? 'on' : ''} onClick={() => setCombo(4)}>4</button>
							<button className={combo === 5 ? 'on' : ''} onClick={() => setCombo(5)}>5</button>
						</div>
					</div>
					<div className="field">
						<label>Match ≥</label>
						<input className="num-in" type="number" min="1" max="5" value={match} onChange={e => setMatch(e.target.value)} />
					</div>
					<div className="opt-spacer"></div>
					<button className="gbtn primary" onClick={optimize}><Icon name="zap" size={13} fill /> Optimize</button>
				</div>

				<div className="attr-panel">
					<div className="attr-hint">
						<div className="txt">Click to cycle: <b className="max">maximize</b> → <b className="exc">exclude</b> → off</div>
						<div className="targets"><b>{targets}</b> targets</div>
					</div>
					<div className="attr-group">
						<div className="gl">Basic</div>
						<div className="chips">
							{B.ATTR_BASIC.map(a => (
								<button key={a} className={chipClass(a)} onClick={() => cycle(a)}>
									{states[a] && <span className="st-dot"></span>}{a}
								</button>
							))}
						</div>
					</div>
					<div className="attr-group">
						<div className="gl">Special 极</div>
						<div className="chips">
							{B.ATTR_SPECIAL.map(a => (
								<button key={a} className={chipClass(a)} onClick={() => cycle(a)}>
									{states[a] && <span className="st-dot"></span>}{a}
								</button>
							))}
						</div>
					</div>
				</div>

				{results && (
					<div className="opt-results screen-anim">
						<div className="rl">{results.length} optimal builds · combo {combo}</div>
						{results.map(r => (
							<div className="mset" key={r.rank}>
								<span className="mrank">#{r.rank}</span>
								<div className="mslots">
									{Array.from({ length: combo }).map((_, i) => (
										<div className="mslot" key={i}><Icon name={['zap', 'spark', 'sword', 'shield', 'heart'][i % 5]} size={13} /></div>
									))}
								</div>
								{r.tag && <span className="mtag">{r.tag}</span>}
								<div className="mscore"><div className="v">{r.score.toLocaleString()}</div><div className="l">score</div></div>
							</div>
						))}
					</div>
				)}
			</div>
		</div>
	);
}

/* ---------- Encounter history ---------- */
function HistoryScreen({ onOpen }) {
	const [list, setList] = React.useState(B.HISTORY);
	function del(id, e) { e.stopPropagation(); setList(l => l.filter(x => x.id !== id)); }
	return (
		<div className="screen screen-anim">
			<div className="shead">
				<span className="shead-title">
					<h2><Icon name="history" size={15} style={{ color: 'var(--ac)' }} /> Encounter History</h2>
					<span className="sub">{list.length} sessions logged</span>
				</span>
				<div className="shead-spacer"></div>
				<button className="gbtn"><Icon name="refresh" size={13} /></button>
				<button className="gbtn danger" onClick={() => setList([])}><Icon name="trash" size={13} /> Clear</button>
			</div>
			<div className="hist-body">
				{list.length === 0 && (
					<div className="empty"><div className="ico"><Icon name="history" size={20} /></div><p>No encounters logged yet</p><span className="mono">Enter combat to begin</span></div>
				)}
				{list.map(h => {
					const tot = fmt(h.total);
					return (
						<div className="enc" key={h.id} onClick={() => onOpen && onOpen()}>
							<div className="enc-time"><div className="t">{h.t}</div><div className="d">{h.date}</div></div>
							<div className="enc-id">
								<div className="nm"><span className="zone"></span>{h.zone || `Map: ${h.map}`}</div>
								<div className="meta"><b>{h.players}</b> players · top <b>{h.top}</b></div>
							</div>
							<div className="enc-score">
								<div className="v">{(h.dps / (h.dpsU === 'm/s' ? 1e6 : 1e3)).toFixed(1)}<em>{h.dpsU}</em></div>
								<div className="tot">{tot.v}{tot.u || h.totU} total</div>
							</div>
							<button className="enc-del" onClick={e => del(h.id, e)}><Icon name="trash" size={14} /></button>
						</div>
					);
				})}
			</div>
		</div>
	);
}

/* ---------- Chat ---------- */
function ChatScreen() {
	const [chan, setChan] = React.useState('all');
	const channels = B.CHANNELS;
	const colorFor = (au) => B.NAME_COLORS[[...au].reduce((a, c) => a + c.charCodeAt(0), 0) % B.NAME_COLORS.length];
	const msgs = chan === 'all' ? B.MESSAGES : B.MESSAGES.filter(m => m.ch === chan);
	const chanMeta = (id) => channels.find(c => c.id === id) || { label: id, color: 'var(--tx-2)' };

	return (
		<div className="screen screen-anim">
			<div className="chat-channels">
				<button className={`cch ${chan === 'all' ? 'on' : ''}`} onClick={() => setChan('all')}>All</button>
				{channels.map(c => (
					<button key={c.id} className={`cch ${chan === c.id ? 'on' : ''}`} onClick={() => setChan(c.id)}>
						<span className="cdot" style={{ background: c.color }}></span>{c.label}
					</button>
				))}
				<div className="chat-tools">
					<button className="tool" title="Channel settings"><Icon name="gear" size={15} /></button>
					<button className="tool danger" title="Clear"><Icon name="trash" size={15} /></button>
				</div>
			</div>
			<div className="chat-body">
				{msgs.map((m, i) => {
					const cm = chanMeta(m.ch);
					return (
						<div className="msg" key={i}>
							<span className="ts">{m.ts}</span>
							<div className="body">
								{chan === 'all' && <span className="chan-pill" style={{ background: `color-mix(in oklab, ${cm.color} 18%, transparent)`, color: cm.color }}>{cm.label}</span>}
								<span className="au" style={{ color: colorFor(m.au) }}>{m.au}:</span>
								{m.tx}
							</div>
						</div>
					);
				})}
			</div>
			<div className="chat-compose">
				<input className="inp" placeholder="Message…  (read-only mirror)" disabled />
				<button className="send"><Icon name="send" size={15} fill /></button>
			</div>
		</div>
	);
}

Object.assign(window, { ModulesScreen, HistoryScreen, ChatScreen });
