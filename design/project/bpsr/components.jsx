/* ============================================================
   BPSR Logs — shared components & helpers
   ============================================================ */
const { useState, useEffect, useRef } = React;
const B = window.BPSR;

/* ---- number formatting ---- */
function fmt(n) {
	if (n >= 1e6) return { v: (n / 1e6).toFixed(n >= 1e7 ? 1 : 2).replace(/\.?0+$/, ''), u: 'm' };
	if (n >= 1e3) return { v: (n / 1e3).toFixed(n >= 1e5 ? 0 : 1).replace(/\.0$/, ''), u: 'k' };
	return { v: String(Math.round(n)), u: '' };
}
function Num({ n, cls }) {
	const f = fmt(n);
	return <span className={cls}><span className="big">{f.v}{f.u && <em>{f.u}</em>}</span></span>;
}

/* ---- icon ---- */
function Icon({ name, size = 16, sw = 2, fill = false, style }) {
	const d = B.ICONS[name];
	if (!d) return null;
	return (
		<svg width={size} height={size} viewBox="0 0 24 24" fill={fill ? 'currentColor' : 'none'}
			stroke={fill ? 'none' : 'currentColor'} strokeWidth={sw} strokeLinecap="round" strokeLinejoin="round" style={style}>
			{d.split('M').filter(Boolean).map((seg, i) => <path key={i} d={'M' + seg} />)}
		</svg>
	);
}

/* ---- class emblem ---- */
function Emblem({ cls, style }) {
	const info = B.CLASSES[cls] || { color: 'var(--tx-2)', role: 'dps' };
	const glyph = B.ROLE_GLYPH[info.role] || 'sword';
	return (
		<div className="emblem" style={{ '--rc': info.color, ...style }}>
			<span className="glyph" style={{ color: info.color, display: 'grid', placeItems: 'center' }}>
				<Icon name={glyph} size={glyph === 'shield' ? 12 : 11} sw={2.2} fill={glyph === 'shield'} />
			</span>
		</div>
	);
}

/* ---- tiny tooltip wrapper (title attr is fine for prototype) ---- */
function Tool({ name, size = 16, sw = 2, title, on, danger, onClick, fill }) {
	return (
		<button className={`tool ${on ? 'on' : ''} ${danger ? 'danger' : ''}`} title={title} onClick={onClick}>
			<Icon name={name} size={size} sw={sw} fill={fill} />
		</button>
	);
}

/* ---- live combat clock (isolated so screens don't remount each tick) ---- */
function LiveClock({ paused }) {
	const [s, setS] = useState(11);
	useEffect(() => {
		if (paused) return;
		const id = setInterval(() => setS(x => x + 1), 1000);
		return () => clearInterval(id);
	}, [paused]);
	const t = `${String(Math.floor(s / 60)).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;
	return (
		<div className="combat-timer">
			<span className={`live-dot ${paused ? 'paused' : ''}`}></span>
			{t}
		</div>
	);
}

/* ============================================================
   Title bar
   ============================================================ */
function TitleBar({ paused, setPaused, totDmg, totDps, density, setDensity, clickthrough, setClickthrough, onSettings, settingsOpen }) {
	return (
		<div className="titlebar">
			<div className="brand">
				<div className="brand-mark">
					<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#04181c" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
						<path d="M3 12h4l2.5 7 5-14L17 12h4" />
					</svg>
				</div>
				<div>
					<div className="brand-name">BPSR<b> Logs</b></div>
				</div>
			</div>

			<div className="combat">
				<LiveClock paused={paused} />
				<div className="tstat">
					<span className="k">T.DMG</span>
					<span className="v">{(() => { const f = fmt(totDmg); return <>{f.v}<em>{f.u}</em></>; })()}</span>
				</div>
				<div className="tstat">
					<span className="k">T.DPS</span>
					<span className="v">{(() => { const f = fmt(totDps); return <>{f.v}<em>{f.u}</em></>; })()}</span>
				</div>
			</div>

			<div className="titlebar-spacer"></div>

			<div className="win-tools">
				<Tool name="camera" title="Screenshot" />
				<Tool name={paused ? 'play' : 'pause'} fill={paused} sw={paused ? 1 : 2} title={paused ? 'Resume' : 'Pause'} onClick={() => setPaused(!paused)} />
				<Tool name="hand" title="Click-through" on={clickthrough} onClick={() => setClickthrough(!clickthrough)} />
				<div className="div"></div>
				<button className={`tool ${density === 'compact' ? 'on' : ''}`} title="Toggle density"
					onClick={() => setDensity(density === 'compact' ? 'expanded' : 'compact')}>
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
						{density === 'compact'
							? <><path d="M4 6h16M4 10h16M4 14h16M4 18h16" /></>
							: <><path d="M4 6h16M4 12h16M4 18h16" /></>}
					</svg>
				</button>
				<Tool name="gear" title="Settings" on={settingsOpen} onClick={onSettings} />
				<div className="div"></div>
				<Tool name="min" title="Minimize" />
				<Tool name="close" title="Close" danger />
			</div>
		</div>
	);
}

/* ============================================================
   Footer tab bar
   ============================================================ */
const TABS = [
	{ id: 'dps',     label: 'DPS',     icon: 'chart' },
	{ id: 'heal',    label: 'Heal',    icon: 'heart' },
	{ id: 'modules', label: 'Modules', icon: 'grid' },
	{ id: 'history', label: 'History', icon: 'history' },
	{ id: 'chat',    label: 'Chat',    icon: 'chat', badge: 3 }
];
function TabBar({ active, setActive }) {
	return (
		<div className="tabbar">
			{TABS.map(t => (
				<button key={t.id} className={`tab ${active === t.id ? 'active' : ''}`} onClick={() => setActive(t.id)}>
					<Icon name={t.icon} size={14} sw={2} fill={t.icon === 'heart' && active === t.id} />
					{t.label}
					{t.badge && active !== t.id && <span className="badge">{t.badge}</span>}
				</button>
			))}
			<div className="tabbar-spacer"></div>
			<div className="tabbar-ver"><span className="pip"></span>BPSR Logs v0.26.0</div>
		</div>
	);
}

Object.assign(window, { fmt, Num, Icon, Emblem, Tool, TitleBar, TabBar, TABS, LiveClock });
