//! Memory tab: every process by RAM, grouped by app bundle, plus "orphans" — dev processes whose
//! terminal / editor / agent is gone (parent is now launchd) and that nobody will ever stop.
use std::collections::{BTreeSet, HashMap};

use serde_json::{json, Value};

use crate::procs;

#[derive(Clone, Debug, Default)]
pub struct MProc {
    pub pid: i32,
    pub ppid: i32,
    pub pgid: i32,
    pub uid: u32,
    pub rss_kb: u64,
    pub cpu: f32,
    pub secs: u64,
    pub exe: String,
    pub args: String,
}

/// `ps -axo pid=,ppid=,pgid=,uid=,rss=,%cpu=,etime=,comm=`
pub fn parse_ps(text: &str) -> HashMap<i32, MProc> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (Some(pid), Some(ppid), Some(pgid), Some(uid), Some(rss), Some(cpu), Some(et)) =
            (it.next(), it.next(), it.next(), it.next(), it.next(), it.next(), it.next())
        else {
            continue;
        };
        let Ok(pid) = pid.parse::<i32>() else { continue };
        let exe = it.collect::<Vec<_>>().join(" ");
        out.insert(
            pid,
            MProc {
                pid,
                ppid: ppid.parse().unwrap_or(0),
                pgid: pgid.parse().unwrap_or(0),
                uid: uid.parse().unwrap_or(0),
                rss_kb: rss.parse().unwrap_or(0),
                cpu: cpu.replace(',', ".").parse().unwrap_or(0.0),
                secs: procs::etime(et),
                exe,
                args: String::new(),
            },
        );
    }
    out
}

/// `ps -axo pid=,args=` → pid → full command line
pub fn parse_args(text: &str) -> HashMap<i32, String> {
    text.lines()
        .filter_map(|l| {
            let t = l.trim_start();
            let (p, a) = t.split_once(' ').unwrap_or((t, ""));
            Some((p.parse().ok()?, a.trim().to_string()))
        })
        .collect()
}

fn run(cmd: &str, args: &[&str]) -> String {
    std::process::Command::new(cmd).args(args).output().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default()
}

pub fn table() -> HashMap<i32, MProc> {
    let mut t = parse_ps(&run("ps", &["-axo", "pid=,ppid=,pgid=,uid=,rss=,%cpu=,etime=,comm="]));
    for (pid, a) in parse_args(&run("ps", &["-axo", "pid=,args="])) {
        if let Some(p) = t.get_mut(&pid) {
            p.args = a;
        }
    }
    t
}

/// RAM in use like Activity Monitor's "Memory Used": app (anonymous − purgeable) + wired + compressed. Bytes.
pub fn parse_vm_stat(text: &str) -> u64 {
    let page = text
        .lines()
        .next()
        .and_then(|l| l.split("page size of ").nth(1))
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(16384);
    let get = |key: &str| -> u64 {
        text.lines()
            .find(|l| l.trim_start().starts_with(key))
            .and_then(|l| l.rsplit(':').next())
            .and_then(|v| v.trim().trim_end_matches('.').parse().ok())
            .unwrap_or(0)
    };
    let app = get("Anonymous pages").saturating_sub(get("Pages purgeable"));
    (app + get("Pages wired down") + get("Pages occupied by compressor")) * page
}

/// `/Applications/Google Chrome.app/Contents/Frameworks/…Helper.app/…` → `/Applications/Google Chrome.app`
pub fn bundle_of(exe: &str) -> Option<&str> {
    let i = exe.find(".app/").or_else(|| exe.ends_with(".app").then(|| exe.len() - 4))?;
    Some(&exe[..i + 4])
}

fn stem(bundle: &str) -> &str {
    bundle.rsplit('/').next().unwrap_or(bundle).trim_end_matches(".app")
}

fn in_applications(bundle: &str) -> bool {
    bundle.starts_with("/Applications/") || bundle.starts_with("/System/Applications/") || bundle.contains("/Applications/")
}

const RUNTIMES: [&str; 14] = ["node", "python", "ruby", "java", "php", "bun", "deno", "esbuild", "go", "cargo", "uvicorn", "gunicorn", "dotnet", "watchman"];

fn is_runtime(exe: &str) -> bool {
    let base = exe.rsplit('/').next().unwrap_or(exe).to_lowercase();
    RUNTIMES.iter().any(|r| base == *r || (base.starts_with(r) && base[r.len()..].chars().all(|c| c.is_ascii_digit() || c == '.')))
}

/// Test browsers (inside an .app outside /Applications) and agent CLIs whose terminal closed.
fn leftover_hint(exe: &str) -> Option<&'static str> {
    let l = exe.to_lowercase();
    let base = l.rsplit('/').next().unwrap_or(&l);
    if l.contains("ms-playwright") {
        Some("Playwright")
    } else if l.contains("puppeteer") {
        Some("Puppeteer")
    } else if l.contains("chrome for testing") || l.contains("/chromium.app/") {
        Some("a test run")
    } else if base == "claude" {
        Some("Claude Code")
    } else if base == "codex" {
        Some("Codex")
    } else {
        None
    }
}

pub fn is_system_proc(p: &MProc, me: u32) -> bool {
    p.uid != me || p.pid <= 1 || procs::is_system(&p.exe, "/Users") && !bundle_of(&p.exe).is_some_and(in_applications)
}

/// A dev process nobody owns any more: ours, parent is launchd, and either a language runtime or a test browser /
/// agent CLI living outside /Applications. Apps from the Dock also have ppid 1, so they never count.
pub fn is_orphan(p: &MProc, me: u32) -> bool {
    if p.uid != me || p.ppid != 1 || is_system_proc(p, me) {
        return false;
    }
    match bundle_of(&p.exe) {
        Some(b) => !in_applications(b) && leftover_hint(&p.exe).is_some(),
        None => is_runtime(&p.exe) || leftover_hint(&p.exe).is_some(),
    }
}

/// "node /x/node_modules/.bin/vite --port 5174" → "node vite"; "python3 -m http.server 8765" → "python3 -m http.server";
/// "node /x/tsc --watch" → "node tsc --watch"
pub fn short_name(p: &MProc) -> String {
    let base = p.exe.rsplit('/').next().unwrap_or(&p.exe).to_string();
    if !is_runtime(&p.exe) {
        return procs::starter(&p.exe).filter(|_| matches!(base.as_str(), "claude" | "codex")).map(str::to_string).unwrap_or(base);
    }
    let words: Vec<String> = p
        .args
        .split_whitespace()
        .skip(1)
        .filter(|w| !w.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .filter(|w| !w.starts_with('-') || ["-m", "--watch", "-w"].contains(w))
        .take(2)
        .map(|w| if w.starts_with('-') { w.to_string() } else { w.rsplit('/').next().unwrap_or(w).trim_end_matches(".js").trim_end_matches(".mjs").to_string() })
        .collect();
    if words.is_empty() { base } else { format!("{base} {}", words.join(" ")) }
}

#[derive(Debug, Default)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub kind: &'static str, // App · Background · Dev · System
    pub pids: Vec<i32>,
    pub kb: u64,
    pub cpu: f32,
    pub secs: u64,
    pub ports: BTreeSet<u16>,
    pub system: bool,
    pub orphan: bool,
    pub left_by: Option<String>,
}

/// One row per app bundle (all helpers summed), one row per other process. Orphans always get their own row.
pub fn group(t: &HashMap<i32, MProc>, me: u32, ports: &HashMap<i32, BTreeSet<u16>>, protect: &[i32], left: &HashMap<i32, String>) -> Vec<Group> {
    let mut by: HashMap<String, Group> = HashMap::new();
    for p in t.values() {
        if p.rss_kb == 0 || protect.contains(&p.pid) {
            continue;
        }
        let system = is_system_proc(p, me);
        let orphan = !system && is_orphan(p, me);
        let bundle = bundle_of(&p.exe).filter(|_| !orphan);
        // every daemon inside a booted iOS runtime (routined, pasted, photoanalysisd…) is one "iOS Simulator" row
        let sim = !system && p.exe.contains("/CoreSimulator/");
        let (id, name, kind) = match bundle {
            _ if sim => ("sim".to_string(), "iOS Simulator".to_string(), "App"),
            _ if system => (bundle.map(|b| format!("b:{b}")).unwrap_or(format!("p:{}", p.pid)), bundle.map(stem).unwrap_or(&short_name(p)).to_string(), "System"),
            Some(b) => {
                let name = procs::starter(&p.exe).filter(|_| !in_applications(b)).map(str::to_string).unwrap_or_else(|| stem(b).to_string());
                (format!("b:{b}"), name, if in_applications(b) { "App" } else { "Background" })
            }
            None if ports.contains_key(&p.pid) || is_runtime(&p.exe) => (format!("p:{}", p.pid), short_name(p), "Dev"),
            None => (format!("p:{}", p.pid), short_name(p), "Background"),
        };
        let g = by.entry(id.clone()).or_insert_with(|| Group { id, name, kind, system, orphan, ..Default::default() });
        g.pids.push(p.pid);
        g.kb += p.rss_kb;
        g.cpu += p.cpu;
        g.secs = g.secs.max(p.secs);
        if let Some(ps) = ports.get(&p.pid) {
            g.ports.extend(ps);
        }
        if orphan {
            g.left_by = left.get(&p.pid).cloned().or_else(|| leftover_hint(&p.exe).map(str::to_string));
        }
    }
    let mut out: Vec<Group> = by.into_values().collect();
    for g in &mut out {
        g.pids.sort();
    }
    out.sort_by(|a, b| b.kb.cmp(&a.kb).then(a.name.cmp(&b.name)));
    out
}

fn base64(b: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(b.len().div_ceil(3) * 4);
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            out.push(if i <= c.len() { T[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    out
}

/// The app's own icon as a 64 px PNG data URL (`b:/Applications/X.app` group ids only), cached in ~/Library/Caches.
/// None when the app ships no .icns (asset-catalog-only apps) — the UI draws a coloured tile instead.
pub fn icon(id: &str) -> Option<String> {
    let bundle = id.strip_prefix("b:")?;
    let home = std::env::var("HOME").ok()?;
    let dir = std::path::Path::new(&home).join("Library/Caches/server-hub/icons");
    let key: String = bundle.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    let png = dir.join(format!("{key}.png"));
    if !png.exists() {
        let plist = format!("{bundle}/Contents/Info.plist");
        let name = run("/usr/bin/plutil", &["-extract", "CFBundleIconFile", "raw", &plist]);
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        let file = if name.ends_with(".icns") { name.to_string() } else { format!("{name}.icns") };
        let icns = format!("{bundle}/Contents/Resources/{file}");
        std::fs::create_dir_all(&dir).ok()?;
        run("/usr/bin/sips", &["-s", "format", "png", "-Z", "64", &icns, "--out", png.to_str()?]);
    }
    std::fs::read(&png).ok().filter(|b| !b.is_empty()).map(|b| format!("data:image/png;base64,{}", base64(&b)))
}

pub fn pressure() -> &'static str {
    match run("sysctl", &["-n", "kern.memorystatus_vm_pressure_level"]).trim() {
        "4" => "critical",
        "2" => "warning",
        _ => "normal",
    }
}

pub fn total_bytes() -> u64 {
    run("sysctl", &["-n", "hw.memsize"]).trim().parse().unwrap_or(0)
}

pub fn used_bytes() -> u64 {
    parse_vm_stat(&run("vm_stat", &[]))
}

pub fn to_json(g: &Group) -> Value {
    json!({
        "id": g.id, "name": g.name, "kind": g.kind, "pids": g.pids, "count": g.pids.len(),
        "mb": g.kb / 1024, "cpu": (g.cpu * 10.0).round() / 10.0, "secs": g.secs, "ports": g.ports,
        "system": g.system, "orphan": g.orphan, "leftBy": g.left_by,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PS: &str = "    1     0     1     0  9000  0.0 3-00:00:00 /sbin/launchd
  100     1   100   501 900000  5.0    05:00:00 /Applications/Google Chrome.app/Contents/MacOS/Google Chrome
  101   100   100   501 600000  1.0    05:00:00 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/1/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)
  200     1   200   501 210000  0.0 2-00:00:00 /opt/homebrew/bin/node
  300     1   300   501 640000  0.0 1-00:00:00 /Users/me/Library/Caches/ms-playwright/chromium-1140/chrome-mac/Chromium.app/Contents/MacOS/Chromium
  400   399   400   501  50000  0.0    01:00 /opt/homebrew/bin/node
  500     1   500     0  80000  0.0 1-00:00:00 /usr/libexec/rapportd
  600     1   600   501 120000  0.0 2-00:00:00 /Applications/Dropbox.app/Contents/MacOS/Dropbox
  700     1   700   501  30000  0.0    03:00:00 /opt/homebrew/bin/redis-server
  800     1   800   501   3000  0.0    03:00:00 /Users/me/.codex/bin/Updater
  900     1   900   501  90000  0.0    03:00:00 /Users/me/.local/bin/claude
  950   949   950   501  36000  0.0    07:00 /Library/Developer/CoreSimulator/Volumes/iOS_23F77/Library/Developer/CoreSimulator/Profiles/Runtimes/iOS 26.5.simruntime/Contents/Resources/RuntimeRoot/usr/libexec/routined
  951   949   951   501  52000  0.0    07:00 /Library/Developer/CoreSimulator/Volumes/iOS_23F77/Library/Developer/CoreSimulator/Profiles/Runtimes/iOS 26.5.simruntime/Contents/Resources/RuntimeRoot/System/Library/PrivateFrameworks/Pasteboard.framework/Support/pasted
";

    fn t() -> HashMap<i32, MProc> {
        let mut t = parse_ps(PS);
        t.get_mut(&200).unwrap().args = "node /x/node_modules/.bin/vite --port 5174".into();
        t.get_mut(&400).unwrap().args = "node server.mjs 3000".into();
        t
    }

    #[test]
    fn parses_ps_and_args() {
        let t = t();
        assert_eq!(t[&100].uid, 501);
        assert_eq!(t[&100].rss_kb, 900000);
        assert_eq!(t[&100].cpu, 5.0);
        assert_eq!(t[&100].exe, "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome");
        assert_eq!(parse_args("  42 python3 -m http.server 8765\n")[&42], "python3 -m http.server 8765");
    }

    #[test]
    fn vm_stat_used() {
        let v = "Mach Virtual Memory Statistics: (page size of 16384 bytes)\nPages wired down:  10.\nPages purgeable:  2.\nAnonymous pages:  12.\nPages occupied by compressor:  5.\n";
        assert_eq!(parse_vm_stat(v), (10 + 10 + 5) * 16384);
    }

    #[test]
    fn bundles() {
        let h = "/Applications/Google Chrome.app/Contents/Frameworks/X.framework/Helpers/Helper (Renderer).app/Contents/MacOS/Helper";
        assert_eq!(bundle_of(h), Some("/Applications/Google Chrome.app"));
        assert_eq!(bundle_of("/opt/homebrew/bin/node"), None);
    }

    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
    }

    #[test]
    fn orphans_are_dev_leftovers_only() {
        let t = t();
        assert!(is_orphan(&t[&200], 501), "node with launchd parent");
        assert!(is_orphan(&t[&300], 501), "playwright chromium");
        assert!(!is_orphan(&t[&400], 501), "node whose terminal is alive");
        assert!(!is_orphan(&t[&600], 501), "Dropbox is a normal app");
        assert!(!is_orphan(&t[&100], 501), "Chrome from the Dock");
        assert!(!is_orphan(&t[&700], 501), "brew services are on purpose");
        assert!(!is_orphan(&t[&500], 501), "root process");
        assert!(!is_orphan(&t[&800], 501), "an updater is not a leftover just because its path says codex");
        assert!(is_orphan(&t[&900], 501), "claude CLI whose terminal closed");
        assert_eq!(short_name(&t[&900]), "Claude Code");
    }

    #[test]
    fn groups_apps_and_names_dev() {
        let t = t();
        let ports = HashMap::from([(400, BTreeSet::from([3000u16]))]);
        let left = HashMap::from([(200, "Claude Code".to_string())]);
        let g = group(&t, 501, &ports, &[], &left);
        let chrome = g.iter().find(|g| g.name == "Google Chrome").unwrap();
        assert_eq!(chrome.pids, vec![100, 101]);
        assert_eq!(chrome.kb, 1_500_000);
        assert_eq!(chrome.kind, "App");
        let vite = g.iter().find(|g| g.pids == vec![200]).unwrap();
        assert_eq!(vite.name, "node vite");
        assert!(vite.orphan);
        assert_eq!(vite.left_by.as_deref(), Some("Claude Code"));
        let pw = g.iter().find(|g| g.pids == vec![300]).unwrap();
        assert!(pw.orphan && pw.left_by.as_deref() == Some("Playwright"));
        let dev = g.iter().find(|g| g.pids == vec![400]).unwrap();
        assert_eq!((dev.kind, dev.name.as_str()), ("Dev", "node server"));
        assert!(g.iter().find(|g| g.pids == vec![500]).unwrap().system);
        assert!(g[0].kb >= g[1].kb, "sorted by memory");
        let sim = g.iter().find(|g| g.name == "iOS Simulator").unwrap();
        assert_eq!((sim.pids.clone(), sim.kind, sim.system), (vec![950, 951], "App", false));
        assert!(group(&t, 501, &ports, &[100, 101], &left).iter().all(|g| g.name != "Google Chrome"), "protected pids hidden");
    }
}
