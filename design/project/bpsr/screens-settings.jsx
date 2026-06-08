/* ============================================================
   BPSR Logs — Settings
   ============================================================ */
function Toggle({ on, onClick }) { return <button className={`tgl ${on ? 'on' : ''}`} onClick={onClick}></button>; }

function SRow({ lab, des, children }) {
	return (
		<div className="srow">
			<div className="si"><div className="lab">{lab}</div>{des && <div className="des">{des}</div>}</div>
			{children}
		</div>
	);
}

function SettingsScreen({ opacity, setOpacity, onClose }) {
	const [tab, setTab] = React.useState('General');
	const [tg, setTg] = React.useState({ bpTimer: true, discord: false, smooth: true, sticky: false, stream: false, autohide: true, highlightYou: true, soundAlert: false });
	const flip = k => setTg(s => ({ ...s, [k]: !s[k] }));
	const tabs = ['General', 'Display', 'Capture', 'Shortcuts', 'Misc'];

	return (
		<div className="screen screen-anim">
			<div className="shead" style={{ paddingBottom: 4 }}>
				<span className="shead-title"><h2><Icon name="gear" size={15} style={{ color: 'var(--ac)' }} /> Settings</h2></span>
				<div className="shead-spacer"></div>
				<button className="gbtn" onClick={onClose}><Icon name="back" size={13} /> Close</button>
			</div>
			<div className="set-tabs">
				{tabs.map(t => <button key={t} className={`stab ${tab === t ? 'on' : ''}`} onClick={() => setTab(t)}>{t}</button>)}
			</div>

			<div className="set-body">
				{tab === 'General' && <>
					<div className="scard">
						<h3><Icon name="eye" size={13} className="ch3-ico" /> Appearance</h3>
						<SRow lab="Background opacity" des={<>0% is fully transparent, 100% fully opaque. Default is <b>60%</b>.</>}><span></span></SRow>
						<div className="slider-wrap">
							<input className="slider" type="range" min="0" max="100" value={opacity} onChange={e => setOpacity(+e.target.value)} />
							<span className="slider-val">{opacity}%</span>
						</div>
					</div>
					<div className="scard">
						<h3><Icon name="zap" size={13} className="ch3-ico" /> Integration</h3>
						<SRow lab="BP Timer" des="World Boss & Magical Creature HP data for bptimer.com"><Toggle on={tg.bpTimer} onClick={() => flip('bpTimer')} /></SRow>
						<SRow lab="Discord encounter report" des="Send a JSON dump + screenshot to Discord when an encounter ends. Off by default — uploads party names, classes and scores."><Toggle on={tg.discord} onClick={() => flip('discord')} /></SRow>
					</div>
					<div className="scard">
						<h3><Icon name="chart" size={13} className="ch3-ico" /> Display Options</h3>
						<SRow lab="Show your name" des="Show Your Class replaces your name with your class.">
							<select className="sel"><option>Show Your Name</option><option>Show Your Class</option><option>Hide Your Name</option></select>
						</SRow>
						<SRow lab="Highlight your row" des="Pin a cyan marker on your combatant."><Toggle on={tg.highlightYou} onClick={() => flip('highlightYou')} /></SRow>
					</div>
				</>}

				{tab === 'Display' && <>
					<div className="scard">
						<h3><Icon name="monitor" size={13} className="ch3-ico" /> Window</h3>
						<SRow lab="Always on top" des="Keep the overlay above the game window."><Toggle on={tg.sticky} onClick={() => flip('sticky')} /></SRow>
						<SRow lab="Smooth bar animation" des="Animate damage bars as values change."><Toggle on={tg.smooth} onClick={() => flip('smooth')} /></SRow>
						<SRow lab="Auto-hide out of combat" des="Fade the meter when no combat is detected."><Toggle on={tg.autohide} onClick={() => flip('autohide')} /></SRow>
					</div>
					<div className="scard">
						<h3><Icon name="sliders" size={13} className="ch3-ico" /> Columns</h3>
						<SRow lab="Default density" des="Choose how much detail rows show on launch.">
							<div className="seg"><button className="on">Compact</button><button>Expanded</button></div>
						</SRow>
						<SRow lab="Accent color" des="Signature highlight for your row & key stats.">
							<div style={{ display: 'flex', gap: 6 }}>
								{['#22d3ee', '#34d399', '#b07bff', '#f0556b'].map((c, i) => (
									<button key={c} style={{ width: 22, height: 22, borderRadius: 6, background: c, border: i === 0 ? '2px solid #fff' : '1px solid var(--line-2)', boxShadow: i === 0 ? '0 0 10px -1px ' + c : 'none' }}></button>
								))}
							</div>
						</SRow>
					</div>
				</>}

				{tab === 'Capture' && <>
					<div className="scard">
						<h3><Icon name="camera" size={13} className="ch3-ico" /> Screenshots</h3>
						<SRow lab="Copy to clipboard" des="Place the capture on your clipboard instantly."><Toggle on={true} onClick={() => {}} /></SRow>
						<SRow lab="Include party panel" des="Append the combatant list to the capture."><Toggle on={tg.stream} onClick={() => flip('stream')} /></SRow>
					</div>
					<div className="scard">
						<h3><Icon name="eye" size={13} className="ch3-ico" /> Streamer mode</h3>
						<SRow lab="Mask other names" des="Replace party member names with their class only."><Toggle on={tg.stream} onClick={() => flip('stream')} /></SRow>
					</div>
				</>}

				{tab === 'Shortcuts' && <>
					<div className="scard">
						<h3><Icon name="keyboard" size={13} className="ch3-ico" /> Hotkeys</h3>
						{[['Toggle overlay', 'Ctrl', 'Shift', 'O'], ['Pause / resume', 'Ctrl', 'Shift', 'P'], ['Reset encounter', 'Ctrl', 'Shift', 'R'], ['Screenshot', 'Ctrl', 'Shift', 'S'], ['Click-through', 'Ctrl', 'Shift', 'C']].map(row => (
							<SRow key={row[0]} lab={row[0]}>
								<div style={{ display: 'flex', gap: 4 }}>
									{row.slice(1).map(k => <kbd key={k} style={{ fontFamily: 'var(--mono)', fontSize: 10, fontWeight: 600, color: 'var(--tx-1)', background: 'var(--bg-3)', border: '1px solid var(--line-2)', borderRadius: 5, padding: '3px 7px' }}>{k}</kbd>)}
								</div>
							</SRow>
						))}
					</div>
				</>}

				{tab === 'Misc' && <>
					<div className="scard">
						<h3><Icon name="spark" size={13} className="ch3-ico" /> Alerts</h3>
						<SRow lab="Sound on encounter end" des="Play a soft chime when combat resolves."><Toggle on={tg.soundAlert} onClick={() => flip('soundAlert')} /></SRow>
					</div>
					<div className="scard">
						<h3><Icon name="grid" size={13} className="ch3-ico" /> Data</h3>
						<SRow lab="Auto-clear on new map" des="Reset the meter when you change zones."><Toggle on={tg.autohide} onClick={() => flip('autohide')} /></SRow>
						<SRow lab="Export logs" des="Save raw encounter data as JSON.">
							<button className="gbtn"><Icon name="history" size={13} /> Export</button>
						</SRow>
					</div>
					<p className="set-sub">BPSR Logs v0.26.0 · Tactical HUD edition · packet parser running.</p>
				</>}
			</div>
		</div>
	);
}

Object.assign(window, { SettingsScreen });
