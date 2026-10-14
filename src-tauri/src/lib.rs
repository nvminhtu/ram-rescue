//! The desktop shell: one `api` command that forwards to hub_core::api::App (same calls as `server-hub bridge`),
//! plus a menu bar icon showing RAM in use, the heaviest apps (Quit) and every server (■). Closing the window keeps it
//! in the menu bar, where Guard (Pro) keeps watching and posts macOS notifications with a Quit button.
use hub_core::api::App;
use hub_core::guard::Alert;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, RunEvent, WindowEvent};

#[tauri::command]
async fn api(app: tauri::State<'_, Arc<App>>, cmd: String, args: Value) -> Result<Value, String> {
    // runs on Tauri's async pool; lsof/ps take a few ms
    app.call(&cmd, &args)
}

fn show(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn gb(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
}

fn size(mb: u64) -> String {
    if mb >= 1024 { format!("{:.1} GB", mb as f64 / 1024.0) } else { format!("{mb} MB") }
}

/// Menu bar title: RAM in use, with ⚠︎ when memory pressure is not normal.
fn tray_title(mem: &Value) -> String {
    let warn = if mem["pressure"] == "normal" { "" } else { "⚠︎ " };
    format!("{warn}{}", gb(mem["used"].as_u64().unwrap_or(0)))
}

/// The 3 heaviest apps you may quit.
fn top_apps(mem: &Value) -> Vec<&Value> {
    mem["groups"].as_array().map(|a| a.iter().filter(|g| g["system"] == false).take(3).collect()).unwrap_or_default()
}

/// Menu bar list: every open localhost port (click = open in browser), running rows with ■, open, quit.
fn tray_menu(app: &AppHandle, snap: &Value, mem: &Value, guard: &Value) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    let p = match mem["pressure"].as_str() { Some("warning") => " · Warning", Some("critical") => " · Critical", _ => "" };
    let head = format!("Memory · {} of {}{p}", gb(mem["used"].as_u64().unwrap_or(0)), gb(mem["total"].as_u64().unwrap_or(0)));
    menu.append(&MenuItem::with_id(app, "mhead", head, false, None::<&str>)?)?;
    for g in top_apps(mem) {
        let leak = if g["leakMbH"].is_u64() { "  · leaking" } else { "" };
        let label = format!("Quit {} — {}{leak}", g["name"].as_str().unwrap_or("?"), size(g["mb"].as_u64().unwrap_or(0)));
        menu.append(&MenuItem::with_id(app, format!("qg:{}", g["id"].as_str().unwrap_or("")), label, true, None::<&str>)?)?;
    }
    // Meeting mode (Pro): shown once a list is set up in the app
    if guard["running"] == true || guard["license"]["state"] != "expired" {
        if guard["meetingOn"] == true {
            let n = guard["meetingQuit"].as_array().map(|a| a.len()).unwrap_or(0);
            menu.append(&MenuItem::with_id(app, "meeting:off", format!("End meeting mode — reopen {n} apps"), true, None::<&str>)?)?;
        } else if guard["meetingApps"].as_array().is_some_and(|a| !a.is_empty()) {
            menu.append(&MenuItem::with_id(app, "meeting:on", "Meeting mode — free RAM", true, None::<&str>)?)?;
        }
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    let ports = snap["ports"].as_array().cloned().unwrap_or_default();
    let head = if ports.is_empty() { "No localhost port open".to_string() } else { format!("Localhost · {} open", ports.len()) };
    menu.append(&MenuItem::with_id(app, "lhead", head, false, None::<&str>)?)?;
    for p in &ports {
        let mark = match p["health"].as_str() { Some("healthy") => "🟢", Some("tcp") => "⚪", _ => "🔴" };
        let label = format!("{mark}  :{}  {} · {}", p["port"], p["name"].as_str().unwrap_or("?"), p["tool"].as_str().unwrap_or(""));
        menu.append(&MenuItem::with_id(app, format!("url:http://localhost:{}", p["port"]), label, true, None::<&str>)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    let rows: Vec<&Value> = snap["entries"].as_array().map(|a| a.iter().filter(|r| r["status"] == "running" || r["status"] == "starting").collect()).unwrap_or_default();
    if !rows.is_empty() {
        menu.append(&MenuItem::with_id(app, "head", format!("Projects · {} running", rows.len()), false, None::<&str>)?)?;
    }
    for r in &rows {
        let port = r["ports"].as_array().and_then(|p| p.first()).and_then(|p| p.as_u64()).map(|p| format!("  :{p}")).unwrap_or_default();
        let label = format!("■  {}{port}", r["name"].as_str().unwrap_or("?"));
        menu.append(&MenuItem::with_id(app, format!("stop:{}", r["id"].as_str().unwrap_or("")), label, true, None::<&str>)?)?;
    }
    if !rows.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    menu.append(&MenuItem::with_id(app, "open", "Open RAM Rescue", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "license", "RAM Rescue Pro…", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Quit RAM Rescue", true, None::<&str>)?)?;
    Ok(menu)
}

/// A macOS notification with Quit / Ignore. Quit quits the app; clicking the banner opens RAM Rescue on that app.
fn notify(app: &AppHandle, a: Alert) {
    let (id, title, body) = match a {
        Alert::Pressure { id, name, mb } => (id, "Your Mac is running low on memory".to_string(), format!("{name} is using {}. Quit it to free memory.", size(mb))),
        Alert::Leak { id, name, mb, mb_per_h } => (id, format!("{name} is leaking memory"), format!("{} and growing about {} an hour.", size(mb), size(mb_per_h))),
    };
    let app = app.clone();
    std::thread::spawn(move || {
        let r = mac_notification_sys::Notification::new()
            .title(&title)
            .message(&body)
            .main_button(mac_notification_sys::MainButton::SingleAction("Quit"))
            .close_button("Ignore")
            .wait_for_click(true)
            .send();
        let core = app.state::<Arc<App>>();
        match r {
            Ok(mac_notification_sys::NotificationResponse::ActionButton(_)) => {
                let _ = core.quit_group(&id);
            }
            Ok(mac_notification_sys::NotificationResponse::Click) => {
                core.set_focus(&id);
                let a2 = app.clone();
                let _ = app.run_on_main_thread(move || show(&a2));
            }
            _ => {}
        }
    });
}

pub fn run() {
    let core = Arc::new(App::new(None));
    let app = tauri::Builder::default()
        .manage(core.clone())
        .invoke_handler(tauri::generate_handler![api])
        .setup(move |app| {
            let handle = app.handle().clone();
            let _ = mac_notification_sys::set_application(&app.config().identifier);
            let snap = core.snapshot(true);
            let mem = core.call("memory", &json!({})).unwrap_or_default();
            let guard = core.call("guard", &json!({})).unwrap_or_default();
            let tray = TrayIconBuilder::with_id("hub")
                .icon(app.default_window_icon().cloned().expect("app icon"))
                .title(tray_title(&mem))
                .tooltip("RAM Rescue")
                .menu(&tray_menu(&handle, &snap, &mem, &guard)?)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, ev| {
                    let id = ev.id().as_ref().to_string();
                    match id.as_str() {
                        "open" => show(app),
                        "license" => {
                            app.state::<Arc<App>>().set_focus("__license__");
                            show(app);
                        }
                        "quit" => app.exit(0),
                        _ => {
                            if let Some(url) = id.strip_prefix("url:") {
                                let _ = hub_core::api::open(url, None);
                            } else if let Some(on) = id.strip_prefix("meeting:") {
                                let _ = app.state::<Arc<App>>().call("meeting", &json!({ "on": on == "on" }));
                            } else if let Some(g) = id.strip_prefix("qg:") {
                                let _ = app.state::<Arc<App>>().quit_group(g);
                            } else if let Some(row) = id.strip_prefix("stop:") {
                                let core = app.state::<Arc<App>>();
                                let _ = core.call("stop", &json!({ "id": row }));
                            }
                        }
                    }
                })
                .build(app)?;
            // keep the menu bar in step; rebuild the menu only when what it lists changes (rebuilding closes an open menu).
            // Guard (Pro) runs in the same loop: pressure every 6 s, leaks + orphans once a minute.
            let core = core.clone();
            std::thread::spawn(move || {
                let mut last = String::new();
                let mut guard = guard;
                for tick in 0u64.. {
                    std::thread::sleep(Duration::from_secs(3));
                    let snap = core.snapshot(false);
                    let mem = core.call("memory", &json!({})).unwrap_or_default();
                    let _ = tray.set_title(Some(tray_title(&mem)));
                    // the guard status reads the Keychain: every 30 s is enough for the menu
                    if tick % 10 == 0 {
                        guard = core.call("guard", &json!({})).unwrap_or_default();
                    }
                    let sig = format!(
                        "{:?}{:?}{:?}{}{}{}",
                        snap["entries"].as_array().map(|a| a.iter().filter(|r| r["status"] == "running").map(|r| format!("{}{}", r["id"], r["ports"])).collect::<Vec<_>>()),
                        snap["ports"].as_array().map(|a| a.iter().map(|p| format!("{}{}", p["port"], p["health"])).collect::<Vec<_>>()),
                        top_apps(&mem).iter().map(|g| format!("{}{}{}", g["id"], g["mb"].as_u64().unwrap_or(0) / 256, g["leakMbH"].is_u64())).collect::<Vec<_>>(),
                        mem["pressure"],
                        guard["meetingOn"],
                        guard["meetingApps"]
                    );
                    if sig != last {
                        last = sig;
                        if let Ok(m) = tray_menu(&handle, &snap, &mem, &guard) {
                            let _ = tray.set_menu(Some(m));
                        }
                    }
                    if tick % 2 == 0 {
                        for a in core.guard_check(tick % 20 == 0) {
                            notify(&handle, a);
                        }
                    }
                    if tick % 28_800 == 0 {
                        std::thread::spawn(hub_core::license::recheck); // once a day
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|w, ev| {
            if let WindowEvent::CloseRequested { api, .. } = ev {
                // closing the window keeps RAM Rescue in the menu bar; Quit lives in the tray menu and ⌘Q
                api.prevent_close();
                let _ = w.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building RAM Rescue");
    app.run(|app, ev| {
        if let RunEvent::Reopen { .. } = ev {
            show(app);
        }
    });
}
