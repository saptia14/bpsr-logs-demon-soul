/* ============================================================
   BPSR Logs — root app
   ============================================================ */
function App() {
	const [tab, setTab] = React.useState('dps');
	const [density, setDensity] = React.useState('expanded');
	const [paused, setPaused] = React.useState(false);
	const [clickthrough, setClickthrough] = React.useState(false);
	const [settingsOpen, setSettingsOpen] = React.useState(false);
	const [detail, setDetail] = React.useState(null); // { player, type }
	const [opacity, setOpacity] = React.useState(92);
	const [shape, setShape] = React.useState('portrait');

	const totDmg = B.DPS_PLAYERS.reduce((a, p) => a + p.dmg, 0);
	const totDps = B.DPS_PLAYERS.reduce((a, p) => a + p.dps, 0);

	function openDetail(player) {
		setDetail({ player, type: tab === 'heal' ? 'heal' : 'dps' });
	}

	// what content to show
	let content;
	if (settingsOpen) {
		content = <SettingsScreen opacity={opacity} setOpacity={setOpacity} onClose={() => setSettingsOpen(false)} />;
	} else if (detail) {
		content = <SkillScreen player={detail.player} type={detail.type} onBack={() => setDetail(null)} />;
	} else if (tab === 'dps') {
		content = <DpsScreen density={density} onOpen={openDetail} />;
	} else if (tab === 'heal') {
		content = <HealScreen density={density} onOpen={openDetail} />;
	} else if (tab === 'modules') {
		content = <ModulesScreen />;
	} else if (tab === 'history') {
		content = <HistoryScreen onOpen={() => { setTab('dps'); }} />;
	} else if (tab === 'chat') {
		content = <ChatScreen />;
	}

	const showDensityToggle = !settingsOpen && !detail && (tab === 'dps' || tab === 'heal');

	function changeTab(t) { setSettingsOpen(false); setDetail(null); setTab(t); }

	return (
		<div id="stage">
			<div className="stage-controls">
				<button className={shape === 'portrait' ? 'on' : ''} onClick={() => setShape('portrait')}>
					<svg width="11" height="13" viewBox="0 0 11 13" fill="none" stroke="currentColor" strokeWidth="1.6"><rect x="1" y="1" width="9" height="11" rx="1.5" /></svg>
					Portrait
				</button>
				<button className={shape === 'landscape' ? 'on' : ''} onClick={() => setShape('landscape')}>
					<svg width="13" height="11" viewBox="0 0 13 11" fill="none" stroke="currentColor" strokeWidth="1.6"><rect x="1" y="1" width="11" height="9" rx="1.5" /></svg>
					Landscape
				</button>
			</div>

			<div className={`win win-${shape}`} style={{ background: `color-mix(in oklab, var(--bg-1) ${opacity}%, transparent)` }}>
				<TitleBar
					paused={paused} setPaused={setPaused}
					totDmg={totDmg} totDps={totDps}
					density={density} setDensity={showDensityToggle ? setDensity : () => {}}
					clickthrough={clickthrough} setClickthrough={setClickthrough}
					onSettings={() => { setSettingsOpen(o => !o); setDetail(null); }} settingsOpen={settingsOpen}
				/>
				<div className="win-body">
					{content}
				</div>
				<TabBar active={tab} setActive={changeTab} />
			</div>

			<div className="stage-tag"><span className="dot"></span>BPSR Logs · Tactical HUD overhaul · live overlay preview</div>
		</div>
	);
}

ReactDOM.createRoot(document.getElementById('root')).render(<App />);
