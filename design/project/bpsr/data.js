/* ============================================================
   BPSR Logs — mock data, class system, icon set
   ============================================================ */
(function () {
	// ---- Class system: color + role ----
	// role: dps | tank | healer  -> drives the emblem glyph
	const CLASSES = {
		Stormblade:       { color: 'var(--c-purple)',  role: 'dps',    short: 'Stormblade' },
		Marksman:         { color: 'var(--c-red)',     role: 'dps',    short: 'Marksman' },
		'Twin Striker':   { color: 'var(--c-gold)',    role: 'dps',    short: 'Twin Striker' },
		'Wind Knight':    { color: 'var(--c-cyan)',    role: 'dps',    short: 'Wind Knight' },
		'Frost Mage':     { color: 'var(--c-blue)',    role: 'dps',    short: 'Frost Mage' },
		'Shield Knight':  { color: 'var(--c-amber)',   role: 'tank',   short: 'Shield Knight' },
		'Heavy Guardian': { color: 'var(--c-brown)',   role: 'tank',   short: 'Heavy Guardian' },
		'Beat Performer': { color: 'var(--c-green)',   role: 'healer', short: 'Beat Performer' },
		'Verdant Oracle': { color: 'var(--c-emerald)', role: 'healer', short: 'Verdant Oracle' }
	};

	// ---- SVG icon paths (24x24, stroke) ----
	const ICONS = {
		camera: 'M14.5 4l1.5 2h3a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h3l1.5-2zM12 17a4 4 0 100-8 4 4 0 000 8z',
		timer: 'M12 8v4l2.5 2.5M12 21a8 8 0 100-16 8 8 0 000 16zM9 2h6',
		pause: 'M8 5v14M16 5v14',
		play: 'M6 4l14 8-14 8z',
		hand: 'M7 11V6a1.5 1.5 0 013 0v4m0-4.5a1.5 1.5 0 013 0V10m0-3a1.5 1.5 0 013 0v5a6 6 0 01-6 6h-1.6a4 4 0 01-3-1.4L6 18l-2.2-2.6a1.6 1.6 0 012.3-2.2L8 15',
		moon: 'M21 12.8A9 9 0 1111.2 3a7 7 0 009.8 9.8z',
		gear: 'M12 15a3 3 0 100-6 3 3 0 000 6zM19.4 15a1.6 1.6 0 00.3 1.8l.1.1a2 2 0 11-2.8 2.8l-.1-.1a1.6 1.6 0 00-2.7 1.1V21a2 2 0 11-4 0v-.1A1.6 1.6 0 006 19.4l-.1.1a2 2 0 11-2.8-2.8l.1-.1a1.6 1.6 0 00-1.1-2.7H2a2 2 0 110-4h.1A1.6 1.6 0 003.6 6l-.1-.1a2 2 0 112.8-2.8l.1.1a1.6 1.6 0 002.7-1.1V2a2 2 0 114 0v.1A1.6 1.6 0 0019.4 6l.1-.1a2 2 0 112.8 2.8l-.1.1a1.6 1.6 0 00-1.1 2.7H21a2 2 0 110 4h-.1a1.6 1.6 0 00-1.5 1z',
		min: 'M5 12h14',
		close: 'M6 6l12 12M18 6L6 18',
		home: 'M3 11l9-8 9 8M5 10v10h14V10',
		refresh: 'M21 12a9 9 0 11-3-6.7M21 4v5h-5',
		trash: 'M4 7h16M9 7V5a1 1 0 011-1h4a1 1 0 011 1v2m2 0v12a1 1 0 01-1 1H8a1 1 0 01-1-1V7',
		back: 'M15 18l-6-6 6-6',
		send: 'M4 12l16-7-7 16-2.5-6.5L4 12z',
		sword: 'M14.5 3.5L21 4l-.5 6.5L8 23l-3-3L17.5 7.5zM3 17l4 4M5.5 14.5l4 4',
		bolt: 'M13 2L4 14h6l-1 8 9-12h-6l1-8z',
		chart: 'M4 20V10M10 20V4M16 20v-7M22 20H2',
		grid: 'M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z',
		heart: 'M12 21s-7-4.6-9.5-9C1 9 2.5 5 6 5c2 0 3.2 1.2 4 2.3C10.8 6.2 12 5 14 5c3.5 0 5 4 3.5 7-2.5 4.4-9.5 9-9.5 9z',
		history: 'M12 8v4l3 2M3.05 11a9 9 0 11.5 4M3 4v5h5',
		chat: 'M21 12a8 8 0 01-11.5 7.2L3 21l1.8-6.5A8 8 0 1121 12z',
		eye: 'M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7zM12 15a3 3 0 100-6 3 3 0 000 6z',
		monitor: 'M3 4h18v12H3zM8 20h8M12 16v4',
		keyboard: 'M4 6h16a1 1 0 011 1v9a1 1 0 01-1 1H4a1 1 0 01-1-1V7a1 1 0 011-1zM7 10h0M11 10h0M15 10h0M7 13h10',
		sliders: 'M4 6h10M18 6h2M4 12h2M10 12h10M4 18h12M20 18h0M14 4v4M6 10v4M16 16v4',
		zap: 'M13 2L4 14h6l-1 8 9-12h-6l1-8z',
		check: 'M5 13l4 4L19 7',
		plus: 'M12 5v14M5 12h14',
		spark: 'M12 3v4M12 17v4M3 12h4M17 12h4M6 6l2.5 2.5M15.5 15.5L18 18M18 6l-2.5 2.5M8.5 15.5L6 18',
		shield: 'M12 3l7 3v5c0 4.4-3 7.7-7 9-4-1.3-7-4.6-7-9V6l7-3z'
	};

	// role glyph (drawn inside emblem)
	const ROLE_GLYPH = { dps: 'sword', tank: 'shield', healer: 'plus' };

	// ---- DPS players ----
	const DPS_PLAYERS = [
		{ uid: 1,  name: 'Aria',       cls: 'Stormblade',     gs: '47.4k', dmg: 4655000, dps: 43400, dPct: 31, cr: 42, cdmg: 78, hidden: false, you: true },
		{ uid: 2,  name: 'Norowo',     cls: 'Marksman',       gs: '46.2k', dmg: 4534000, dps: 43100, dPct: 31, cr: 50, cdmg: 86 },
		{ uid: 3,  name: 'Nogi',       cls: 'Twin Striker',   gs: '47.4k', dmg: 2907000, dps: 30800, dPct: 18, cr: 31, cdmg: 74 },
		{ uid: 4,  name: 'Kasai',      cls: 'Wind Knight',    gs: '45.1k', dmg: 1855000, dps: 20200, dPct: 12, cr: 40, cdmg: 71 },
		{ uid: 5,  name: 'Gaby',       cls: 'Frost Mage',     gs: '44.8k', dmg: 1685000, dps: 18100, dPct: 11, cr: 33, cdmg: 66 },
		{ uid: 6,  name: 'ChomiBoy',   cls: 'Marksman',       gs: '43.9k', dmg: 1331000, dps: 13900, dPct: 9,  cr: 28, cdmg: 70 },
		{ uid: 7,  name: 'Valenzia',   cls: 'Shield Knight',  gs: '48.2k', dmg: 907000,  dps: 10800, dPct: 6,  cr: 20, cdmg: 14 },
		{ uid: 8,  name: 'Nova',       cls: 'Heavy Guardian', gs: '42.3k', dmg: 728000,  dps: 8100,  dPct: 5,  cr: 22, cdmg: 39 },
		{ uid: 9,  name: 'Lyric',      cls: 'Beat Performer', gs: '41.0k', dmg: 331000,  dps: 3900,  dPct: 3,  cr: 27, cdmg: 12 },
		{ uid: 10, name: 'Sora',       cls: 'Verdant Oracle', gs: '40.2k', dmg: 198000,  dps: 2400,  dPct: 2,  cr: 20, cdmg: 0 }
	];

	// ---- Heal players ----
	const HEAL_PLAYERS = [
		{ uid: 9,  name: 'Lyric',    cls: 'Beat Performer', gs: '41.0k', heal: 1124000, hps: 12400, hPct: 38, over: 22, cr: 31, you: false },
		{ uid: 10, name: 'Sora',     cls: 'Verdant Oracle', gs: '40.2k', heal: 968000,  hps: 10200, hPct: 33, over: 19, cr: 28 },
		{ uid: 7,  name: 'Valenzia', cls: 'Shield Knight',  gs: '48.2k', heal: 312000,  hps: 3400,  hPct: 11, over: 9,  cr: 14 },
		{ uid: 1,  name: 'Aria',     cls: 'Stormblade',     gs: '47.4k', heal: 268000,  hps: 2900,  hPct: 9,  over: 31, cr: 18, you: true },
		{ uid: 8,  name: 'Nova',     cls: 'Heavy Guardian', gs: '42.3k', heal: 184000,  hps: 2000,  hPct: 6,  over: 12, cr: 22 },
		{ uid: 4,  name: 'Kasai',    cls: 'Wind Knight',    gs: '45.1k', heal: 96000,   hps: 1050,  hPct: 3,  over: 8,  cr: 11 }
	];

	// ---- Skill breakdown per player (keyed by uid) ----
	const SKILLS = {
		1: { // Aria — Stormblade
			skills: [
				{ name: 'Tempest Edge',     sub: 'CHAIN',   total: 1452000, dps: 13500, hits: 184, cr: 46, pct: 31 },
				{ name: 'Voltaic Rush',     sub: 'DASH',    total: 988000,  dps: 9200,  hits: 96,  cr: 41, pct: 21 },
				{ name: 'Storm Brand',      sub: 'DOT',     total: 742000,  dps: 6900,  hits: 320, cr: 12, pct: 16 },
				{ name: 'Thunderclap',      sub: 'AOE',     total: 624000,  dps: 5800,  hits: 48,  cr: 55, pct: 13 },
				{ name: 'Gale Slash',       sub: 'BASIC',   total: 488000,  dps: 4500,  hits: 240, cr: 22, pct: 10 },
				{ name: 'Overcharge',       sub: 'BURST',   total: 361000,  dps: 3360,  hits: 12,  cr: 83, pct: 8  }
			]
		}
	};

	// fallback generic skills for any other player
	const GENERIC_SKILLS = [
		{ name: 'Primary Strike',  sub: 'BASIC', total: 980000, dps: 9100, hits: 210, cr: 30, pct: 34 },
		{ name: 'Heavy Blow',      sub: 'POWER', total: 720000, dps: 6700, hits: 88,  cr: 44, pct: 25 },
		{ name: 'Rend',            sub: 'DOT',   total: 540000, dps: 5000, hits: 260, cr: 14, pct: 19 },
		{ name: 'Finisher',        sub: 'BURST', total: 360000, dps: 3350, hits: 20,  cr: 71, pct: 12 },
		{ name: 'Quick Jab',       sub: 'BASIC', total: 290000, dps: 2700, hits: 180, cr: 19, pct: 10 }
	];

	// ---- Encounter history ----
	const HISTORY = [
		{ id: 1, zone: 'Bahamar Highlands', map: null,  t: '01:42', date: '8 Jun · 12:23 AM', players: 20, top: 'Player 175311312', dps: 115000,  dpsU: 'k/s', total: 518300,    totU: '' },
		{ id: 2, zone: 'Bahamar Highlands', map: null,  t: '02:11', date: '8 Jun · 12:22 AM', players: 26, top: 'Aria',              dps: 102600,  dpsU: 'k/s', total: 1200000,   totU: 'm' },
		{ id: 3, zone: 'Bahamar Highlands', map: null,  t: '02:14', date: '7 Jun · 11:11 PM', players: 5,  top: 'Kasai',             dps: 1700000, dpsU: 'm/s', total: 224400,    totU: 'm' },
		{ id: 4, zone: null,                map: '6524', t: '01:31', date: '7 Jun · 11:08 PM', players: 5,  top: 'Gaby',              dps: 4000000, dpsU: 'm/s', total: 362200,    totU: 'm' },
		{ id: 5, zone: 'Ashen Reliquary',   map: null,  t: '03:48', date: '7 Jun · 10:40 PM', players: 8,  top: 'Norowo',            dps: 89200,   dpsU: 'k/s', total: 2400000,   totU: 'm' },
		{ id: 6, zone: 'Frostvein Caverns', map: null,  t: '00:58', date: '7 Jun · 10:02 PM', players: 4,  top: 'Nogi',              dps: 156000,  dpsU: 'k/s', total: 540000,    totU: '' }
	];

	// ---- Chat ----
	const CHANNELS = [
		{ id: 'union',  label: 'Union',  color: 'var(--ac)' },
		{ id: 'world',  label: 'World',  color: 'var(--c-blue)' },
		{ id: 'local',  label: 'Local',  color: 'var(--good)' },
		{ id: 'team',   label: 'Team',   color: 'var(--c-purple)' },
		{ id: 'group',  label: 'Group',  color: 'var(--c-gold)' },
		{ id: 'system', label: 'System', color: 'var(--tx-2)' }
	];
	const NAME_COLORS = ['var(--c-red)','var(--c-blue)','var(--c-green)','var(--c-gold)','var(--c-purple)','var(--c-cyan)','var(--ac-bright)','var(--c-emerald)'];
	const MESSAGES = [
		{ ch: 'union',  ts: '12:22 AM', au: 'CupcakeAvenged', tx: "Week's about to reset — remember to buy out your shops & finish weeklies." },
		{ ch: 'union',  ts: '12:22 AM', au: 'Sora',           tx: '10 spots left!! Any class welcome.' },
		{ ch: 'world',  ts: '12:22 AM', au: 'gin',            tx: 'is there a way to find the rift where there is no fight?' },
		{ ch: 'world',  ts: '12:22 AM', au: 'CupcakeAvenged', tx: "do it at the time that it isn't fight time" },
		{ ch: 'union',  ts: '12:22 AM', au: 'gin',            tx: 'oh it goes by hour' },
		{ ch: 'union',  ts: '12:23 AM', au: 'Amanie',         tx: "I think it's random. Check the rift near Ruthless Cabbage." },
		{ ch: 'team',   ts: '12:23 AM', au: 'Sora',           tx: 'Still at 10 spots for chao realm boss run! Whose joining??' },
		{ ch: 'world',  ts: '12:24 AM', au: 'Ina',            tx: 'thanks people' },
		{ ch: 'union',  ts: '12:24 AM', au: 'Owl',            tx: 'Raid S1 Dragons before reset applies' },
		{ ch: 'group',  ts: '12:24 AM', au: 'Sam',            tx: 'gin — no fight rifts are pink. Find a pink rift.' },
		{ ch: 'union',  ts: '12:24 AM', au: 'Kuwi',           tx: 'Teyvat [ID 1408] is recruiting! New players + merges welcome.' },
		{ ch: 'local',  ts: '12:25 AM', au: 'Kite',           tx: 'any RK?' },
		{ ch: 'team',   ts: '12:25 AM', au: 'IMBA',           tx: 'need 3 DPS for last floor of stimens' },
		{ ch: 'team',   ts: '12:25 AM', au: 'Sora',           tx: 'Rdy up everyone!! 9 spots left!!' },
		{ ch: 'system', ts: '12:26 AM', au: 'System',         tx: 'Encounter ended — Bahamar Highlands · 02:11 logged.' },
		{ ch: 'union',  ts: '12:26 AM', au: 'Sora',           tx: '8 spots!! We neeed more!!' },
		{ ch: 'world',  ts: '12:26 AM', au: 'gin',            tx: 'thanks Sam — at first i thought u were pulling my leg' }
	];

	// ---- Module optimizer attributes ----
	const ATTR_BASIC = ['Strength Boost','Agility Boost','Intellect Boost','Special Attack','Elite Strike','Healing Boost','Healing Enhance','Resistance','Armor','Cast Focus','Attack SPD','Crit Focus','Luck Focus'];
	const ATTR_SPECIAL = ['DMG Stack','Agile','Life Condense','First Aid','Final Protection','Life Wave','Life Steal','Team Luck&Crit'];

	window.BPSR = {
		CLASSES, ICONS, ROLE_GLYPH, DPS_PLAYERS, HEAL_PLAYERS, SKILLS, GENERIC_SKILLS,
		HISTORY, CHANNELS, NAME_COLORS, MESSAGES, ATTR_BASIC, ATTR_SPECIAL
	};
	window.B = window.BPSR; // shared across all babel scripts
})();
