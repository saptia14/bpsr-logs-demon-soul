//! Process/PID + port-based game-connection detection.
//!
//! Instead of guessing the game server by scanning packet bytes for signatures
//! (fragile, and broken by VPNs that rewrite/encapsulate addressing), we ask
//! Windows which TCP connections belong to the game process — exactly what ZDPS
//! does (`iphlpapi!GetExtendedTcpTable` + exe→PID mapping). A captured TCP flow
//! is "the game" when one of its ports is a local port owned by the game PID.
//!
//! This is VPN/ExitLag-robust (the owning PID is the same regardless of which
//! adapter/IP the tunnel uses) and naturally collects the game's live TCP ports.

use log::info;
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Game executable names to look for (without `.exe`). Mirrors ZDPS's default
/// `["BPSR", "BPSR_STEAM"]` (Standalone + Steam).
pub const DEFAULT_GAME_EXES: &[&str] = &["BPSR", "BPSR_STEAM", "BPSR_EPIC"];

/// How long a refreshed connection snapshot is considered fresh.
const REFRESH_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Debug, Default, Clone)]
pub struct GameConnections {
    /// PIDs of the running game process(es).
    pub pids: HashSet<u32>,
    /// Local TCP ports owned by the game process(es).
    pub local_ports: HashSet<u16>,
}

struct Cached {
    conns: GameConnections,
    refreshed_at: Instant,
}

static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();

fn cache() -> &'static Mutex<Option<Cached>> {
    CACHE.get_or_init(|| Mutex::new(None))
}

/// Current game connections, refreshing from the OS at most once per
/// [`REFRESH_INTERVAL`]. Returns an empty set on non-Windows or on failure.
pub fn game_connections() -> GameConnections {
    let mut guard = cache().lock().unwrap_or_else(|e| e.into_inner());
    let needs_refresh = match guard.as_ref() {
        Some(c) => c.refreshed_at.elapsed() >= REFRESH_INTERVAL,
        None => true,
    };
    if needs_refresh {
        let conns = collect(DEFAULT_GAME_EXES);
        *guard = Some(Cached {
            conns: conns.clone(),
            refreshed_at: Instant::now(),
        });
        conns
    } else {
        guard.as_ref().map(|c| c.conns.clone()).unwrap_or_default()
    }
}

/// True if either endpoint port of a captured flow is a game-owned local port.
pub fn is_game_flow(src_port: u16, dst_port: u16) -> bool {
    let conns = game_connections();
    if conns.local_ports.is_empty() {
        return false;
    }
    conns.local_ports.contains(&src_port) || conns.local_ports.contains(&dst_port)
}

// [CHATDEBUG] Logs the detection result only when it changes (low noise).
static LAST_DETECT_LOG: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new(String::new()));
fn log_detect_change(msg: String) {
    if let Ok(mut last) = LAST_DETECT_LOG.lock() {
        if *last != msg {
            info!("{msg}");
            *last = msg;
        }
    }
}

#[cfg(target_os = "windows")]
fn collect(exe_names: &[&str]) -> GameConnections {
    let pids = windows_impl::game_pids(exe_names);
    if pids.is_empty() {
        log_detect_change(format!(
            "[CHATDEBUG] tcp_table: NO game process found (looking for {exe_names:?}). \
             Secondary connections (e.g. chat) will NOT be tracked."
        ));
        return GameConnections::default();
    }
    let local_ports = windows_impl::owned_local_ports(&pids);
    let mut sorted: Vec<u16> = local_ports.iter().copied().collect();
    sorted.sort_unstable();
    log_detect_change(format!(
        "[CHATDEBUG] tcp_table: game pids={pids:?} local_ports={sorted:?}"
    ));
    GameConnections { pids, local_ports }
}

#[cfg(not(target_os = "windows"))]
fn collect(_exe_names: &[&str]) -> GameConnections {
    GameConnections::default()
}

/// Connection diagnostics for the UI (Settings → Capture).
#[derive(serde::Serialize, specta::Type, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CaptureDiagnostics {
    /// Whether the game process is currently detected.
    pub game_detected: bool,
    /// Number of running game processes found.
    pub process_count: f64,
    /// Game-owned local TCP ports (sorted).
    pub ports: Vec<f64>,
}

pub fn diagnostics() -> CaptureDiagnostics {
    let conns = game_connections();
    let mut ports: Vec<f64> = conns.local_ports.iter().map(|&p| p as f64).collect();
    ports.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    CaptureDiagnostics {
        game_detected: !conns.pids.is_empty(),
        process_count: conns.pids.len() as f64,
        ports,
    }
}

/// Log a one-time summary of detected game connections (diagnostics).
pub fn log_summary() {
    let conns = game_connections();
    info!(
        "Capture diagnostics: {} game process(es), {} local port(s) tracked",
        conns.pids.len(),
        conns.local_ports.len()
    );
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::collections::HashSet;
    use std::ffi::c_void;

    const AF_INET: u32 = 2;
    const TCP_TABLE_OWNER_PID_ALL: u32 = 5;
    const NO_ERROR: u32 = 0;
    const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
    const TH32CS_SNAPPROCESS: u32 = 0x0000_0002;
    const INVALID_HANDLE_VALUE: isize = -1;
    const MAX_PATH: usize = 260;

    #[repr(C)]
    struct MibTcpRowOwnerPid {
        state: u32,
        local_addr: u32,
        local_port: u32, // port is in the low 16 bits, network byte order
        remote_addr: u32,
        remote_port: u32,
        owning_pid: u32,
    }

    #[repr(C)]
    struct ProcessEntry32W {
        dw_size: u32,
        cnt_usage: u32,
        th32_process_id: u32,
        th32_default_heap_id: usize,
        th32_module_id: u32,
        cnt_threads: u32,
        th32_parent_process_id: u32,
        pc_pri_class_base: i32,
        dw_flags: u32,
        sz_exe_file: [u16; MAX_PATH],
    }

    #[link(name = "iphlpapi")]
    unsafe extern "system" {
        fn GetExtendedTcpTable(
            p_tcp_table: *mut c_void,
            pdw_size: *mut u32,
            b_order: i32,
            ul_af: u32,
            table_class: u32,
            reserved: u32,
        ) -> u32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateToolhelp32Snapshot(dw_flags: u32, th32_process_id: u32) -> isize;
        fn Process32FirstW(h_snapshot: isize, lppe: *mut ProcessEntry32W) -> i32;
        fn Process32NextW(h_snapshot: isize, lppe: *mut ProcessEntry32W) -> i32;
        fn CloseHandle(h_object: isize) -> i32;
    }

    /// Resolve PIDs of running processes whose exe name (without extension)
    /// case-insensitively matches one of `exe_names`.
    pub fn game_pids(exe_names: &[&str]) -> HashSet<u32> {
        let mut pids = HashSet::new();
        let wanted: Vec<String> = exe_names.iter().map(|s| s.to_ascii_lowercase()).collect();

        // SAFETY: standard Toolhelp32 process-enumeration sequence. The handle
        // is closed before returning. `entry` is fully initialized (dw_size set)
        // before the first call as the API requires.
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return pids;
            }
            let mut entry: ProcessEntry32W = std::mem::zeroed();
            entry.dw_size = std::mem::size_of::<ProcessEntry32W>() as u32;

            if Process32FirstW(snapshot, &mut entry) != 0 {
                loop {
                    let name = exe_name_lower(&entry.sz_exe_file);
                    if wanted.iter().any(|w| name == *w) {
                        pids.insert(entry.th32_process_id);
                    }
                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
        }
        pids
    }

    /// All local TCP ports (host byte order) owned by any PID in `pids`.
    pub fn owned_local_ports(pids: &HashSet<u32>) -> HashSet<u16> {
        let mut ports = HashSet::new();
        let mut size: u32 = 0;

        // SAFETY: standard two-call GetExtendedTcpTable pattern — first call
        // sizes the buffer, second fills it. We only read `num_entries` rows.
        unsafe {
            let ret = GetExtendedTcpTable(
                std::ptr::null_mut(),
                &mut size,
                0,
                AF_INET,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
            if ret != ERROR_INSUFFICIENT_BUFFER || size == 0 {
                return ports;
            }

            let mut buffer = vec![0u8; size as usize];
            let ret = GetExtendedTcpTable(
                buffer.as_mut_ptr() as *mut c_void,
                &mut size,
                0,
                AF_INET,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
            if ret != NO_ERROR {
                return ports;
            }

            // Layout: DWORD num_entries, followed by num_entries rows.
            let num_entries = u32::from_ne_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
            let row_size = std::mem::size_of::<MibTcpRowOwnerPid>();
            let base = 4usize; // after the leading DWORD (rows are 4-byte aligned)
            for i in 0..num_entries as usize {
                let off = base + i * row_size;
                if off + row_size > buffer.len() {
                    break;
                }
                let row = &*(buffer.as_ptr().add(off) as *const MibTcpRowOwnerPid);
                if pids.contains(&row.owning_pid) {
                    // local_port is network byte order in the low 16 bits.
                    let port = ((row.local_port & 0xff) << 8 | (row.local_port >> 8) & 0xff) as u16;
                    ports.insert(port);
                }
            }
        }
        ports
    }

    fn exe_name_lower(wide: &[u16]) -> String {
        let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
        let name = String::from_utf16_lossy(&wide[..len]);
        let stem = name.strip_suffix(".exe").unwrap_or(&name);
        stem.to_ascii_lowercase()
    }
}
