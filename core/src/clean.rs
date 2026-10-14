//! Clean (free): caches that apps rebuild by themselves — Xcode, simulators, package managers, browsers. A fixed list,
//! nothing else: never cookies, history, passwords, profiles, Archives or a project's files (user files are Sweepy's job).
//! Cleaning moves the folder to the Trash (NSFileManager, no Finder permission needed), so it can be dragged back.
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Item {
    pub id: &'static str,
    pub name: &'static str,
    pub group: &'static str, // Developer · Browsers
    /// Relative to $HOME.
    pub rel: &'static str,
    /// Must not be running while we clean (substring of an executable path).
    pub app: Option<&'static str>,
    pub app_name: Option<&'static str>,
    /// One short line shown under the name.
    pub note: &'static str,
}

pub const ITEMS: &[Item] = &[
    Item { id: "xcode-derived", name: "Xcode build files", group: "Developer", rel: "Library/Developer/Xcode/DerivedData", app: Some("/Xcode.app/"), app_name: Some("Xcode"), note: "DerivedData — rebuilt on the next build" },
    Item { id: "xcode-device", name: "iOS device support", group: "Developer", rel: "Library/Developer/Xcode/iOS DeviceSupport", app: Some("/Xcode.app/"), app_name: Some("Xcode"), note: "Downloaded again when you plug in a device" },
    Item { id: "sim-caches", name: "Simulator caches", group: "Developer", rel: "Library/Developer/CoreSimulator/Caches", app: Some("/Simulator.app/"), app_name: Some("Simulator"), note: "Rebuilt when a simulator boots" },
    Item { id: "npm", name: "npm cache", group: "Developer", rel: ".npm/_cacache", app: None, app_name: None, note: "Packages download again when needed" },
    Item { id: "yarn", name: "Yarn cache", group: "Developer", rel: "Library/Caches/Yarn", app: None, app_name: None, note: "Packages download again when needed" },
    Item { id: "gradle", name: "Gradle cache", group: "Developer", rel: ".gradle/caches", app: Some("/Android Studio.app/"), app_name: Some("Android Studio"), note: "Downloaded again on the next build" },
    Item { id: "cocoapods", name: "CocoaPods cache", group: "Developer", rel: "Library/Caches/CocoaPods", app: None, app_name: None, note: "Pods download again when needed" },
    Item { id: "homebrew", name: "Homebrew downloads", group: "Developer", rel: "Library/Caches/Homebrew", app: None, app_name: None, note: "Old installers Homebrew keeps" },
    Item { id: "pip", name: "pip cache", group: "Developer", rel: "Library/Caches/pip", app: None, app_name: None, note: "Packages download again when needed" },
    Item { id: "playwright", name: "Playwright browsers", group: "Developer", rel: "Library/Caches/ms-playwright", app: None, app_name: None, note: "Run `npx playwright install` before your next test" },
    Item { id: "chrome", name: "Google Chrome cache", group: "Browsers", rel: "Library/Caches/Google/Chrome", app: Some("/Google Chrome.app/"), app_name: Some("Google Chrome"), note: "Cached pages and images — you stay signed in" },
    Item { id: "edge", name: "Microsoft Edge cache", group: "Browsers", rel: "Library/Caches/Microsoft Edge", app: Some("/Microsoft Edge.app/"), app_name: Some("Microsoft Edge"), note: "Cached pages and images — you stay signed in" },
    Item { id: "brave", name: "Brave cache", group: "Browsers", rel: "Library/Caches/BraveSoftware", app: Some("/Brave Browser.app/"), app_name: Some("Brave"), note: "Cached pages and images — you stay signed in" },
    Item { id: "arc", name: "Arc cache", group: "Browsers", rel: "Library/Caches/company.thebrowser.Browser", app: Some("/Arc.app/"), app_name: Some("Arc"), note: "Cached pages and images — you stay signed in" },
    Item { id: "firefox", name: "Firefox cache", group: "Browsers", rel: "Library/Caches/Firefox", app: Some("/Firefox.app/"), app_name: Some("Firefox"), note: "Cached pages and images — you stay signed in" },
    Item { id: "safari", name: "Safari cache", group: "Browsers", rel: "Library/Caches/com.apple.Safari", app: Some("/Safari.app/"), app_name: Some("Safari"), note: "Cached pages and images — you stay signed in" },
];

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
}

/// Size in KB (`du -sk`), None when the folder can't be read (Safari without Full Disk Access).
fn size_kb(p: &Path) -> Option<u64> {
    let o = Command::new("/usr/bin/du").arg("-sk").arg(p).output().ok()?;
    let kb = String::from_utf8_lossy(&o.stdout).split_whitespace().next()?.parse().ok()?;
    // du prints a size even when it could not enter some folders; a denied top folder gives 0 + an error
    if !o.status.success() && kb == 0 {
        return None;
    }
    Some(kb)
}

/// `{ items: [{ id, name, group, note, path, mb, readable, blockedBy }] }` — only what exists on this Mac.
/// `running` = executable paths of running processes.
pub fn list(running: &[String]) -> Value {
    list_in(&home(), running)
}

pub fn list_in(home: &Path, running: &[String]) -> Value {
    let items: Vec<Value> = std::thread::scope(|s| {
        let hs: Vec<_> = ITEMS
            .iter()
            .map(|it| {
                let p = home.join(it.rel);
                s.spawn(move || (it, p.is_dir().then(|| size_kb(&p)), p))
            })
            .collect();
        hs.into_iter()
            .filter_map(|h| h.join().ok())
            .filter_map(|(it, size, p)| {
                let size = size?; // folder missing → not shown
                let blocked = it.app.filter(|a| running.iter().any(|e| e.contains(a))).and(it.app_name);
                Some(json!({
                    "id": it.id, "name": it.name, "group": it.group, "note": it.note,
                    "path": p.to_string_lossy().replacen(&*home.to_string_lossy(), "~", 1),
                    "mb": size.map(|kb| kb / 1024), "readable": size.is_some(), "blockedBy": blocked,
                }))
            })
            .filter(|v| v["mb"].as_u64() != Some(0))
            .collect()
    });
    json!({ "items": items })
}

/// Move the chosen caches to the Trash. Refuses ids not in ITEMS and items whose app is running.
/// → `{ ok, freedMb, done: [ids], skipped: [{ id, why }] }`
pub fn run(ids: &[String], running: &[String]) -> Value {
    let mut ctx = trash::TrashContext::default();
    {
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        ctx.set_delete_method(DeleteMethod::NsFileManager);
    }
    let (mut freed, mut done, mut skipped) = (0u64, vec![], vec![]);
    for id in ids {
        let Some(it) = ITEMS.iter().find(|i| i.id == id) else {
            skipped.push(json!({ "id": id, "why": "unknown" }));
            continue;
        };
        if let Some(name) = it.app.filter(|a| running.iter().any(|e| e.contains(a))).and(it.app_name) {
            skipped.push(json!({ "id": id, "why": format!("Quit {name} first") }));
            continue;
        }
        let p = home().join(it.rel);
        let mb = size_kb(&p).unwrap_or(0) / 1024;
        match ctx.delete(&p) {
            Ok(()) => {
                freed += mb;
                done.push(id.clone());
            }
            Err(e) => skipped.push(json!({ "id": id, "why": e.to_string() })),
        }
    }
    json!({ "ok": true, "freedMb": freed, "done": done, "skipped": skipped })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_rebuildable_caches_are_listed() {
        for it in ITEMS {
            let ok = it.rel.starts_with("Library/Caches/")
                || ["Library/Developer/Xcode/DerivedData", "Library/Developer/Xcode/iOS DeviceSupport", "Library/Developer/CoreSimulator/Caches", ".npm/_cacache", ".gradle/caches"].contains(&it.rel);
            assert!(ok, "{} is not a cache folder", it.rel);
            let l = it.rel.to_lowercase();
            for bad in ["cookie", "history", "password", "keychain", "archives", "node_modules", "pnpm"] {
                assert!(!l.contains(bad), "{} touches {bad}", it.rel);
            }
        }
    }

    #[test]
    fn lists_what_exists_and_blocks_running_browsers() {
        let home = tempfile::tempdir().unwrap();
        let chrome = home.path().join("Library/Caches/Google/Chrome");
        std::fs::create_dir_all(&chrome).unwrap();
        std::fs::write(chrome.join("blob"), vec![1u8; 2 * 1024 * 1024]).unwrap();
        let v = list_in(home.path(), &["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".into()]);
        let items = v["items"].as_array().unwrap();
        assert_eq!(items.len(), 1, "only Chrome exists: {v}");
        assert_eq!(items[0]["id"], "chrome");
        assert_eq!(items[0]["blockedBy"], "Google Chrome");
        assert!(items[0]["mb"].as_u64().unwrap() >= 1);
        assert_eq!(items[0]["path"], "~/Library/Caches/Google/Chrome");
    }

    #[test]
    fn run_refuses_unknown_ids_and_running_apps() {
        let v = run(&["../../etc".into(), "chrome".into()], &["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".into()]);
        assert_eq!(v["done"].as_array().unwrap().len(), 0);
        assert_eq!(v["skipped"][0]["why"], "unknown");
        assert_eq!(v["skipped"][1]["why"], "Quit Google Chrome first");
    }
}
