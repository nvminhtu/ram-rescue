//! Pro (Guard): 14-day trial from the first launch, then a Lemon Squeezy license key ($9.99 once, 2 Macs per key).
//! Trial start and key live in the login Keychain (`security` CLI), so deleting the app's data doesn't reset the trial.
//! Lemon Squeezy is called with /usr/bin/curl — no HTTP library. API: https://docs.lemonsqueezy.com/api/license-api
//! (activate / validate / deactivate, no API key needed). Same design as Markdown Everywhere's license.rs.
//! `SERVER_HUB_LICENSE=trial|expired|licensed` forces a state (screenshots, tests by hand).
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const SERVICE: &str = "app.oneclicktool.serverhub";
const TRIAL_DAYS: i64 = 14;
/// Keys of other products in the same store are refused.
const PRODUCT: &str = "RAM Rescue";
const API: &str = "https://api.lemonsqueezy.com/v1/licenses";
/// A licensed Mac asks Lemon Squeezy again after this long (refunded / disabled keys stop working); offline is fine.
const RECHECK_SECS: i64 = 7 * 86_400;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct Saved {
    pub key: String,
    pub instance: String,
    #[serde(default)]
    pub checked: i64,
}

#[derive(serde::Serialize, Debug, PartialEq)]
pub struct Status {
    /// "licensed" · "trial" · "expired"
    pub state: &'static str,
    #[serde(rename = "daysLeft")]
    pub days_left: i64,
    /// Last 4 characters of the key, for the License window.
    pub key: String,
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn kc_get(account: &str) -> Option<String> {
    let o = Command::new("/usr/bin/security").args(["find-generic-password", "-s", SERVICE, "-a", account, "-w"]).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_string()).filter(|s| !s.is_empty())
}

fn kc_set(account: &str, value: &str) -> Result<(), String> {
    let o = Command::new("/usr/bin/security")
        .args(["add-generic-password", "-U", "-s", SERVICE, "-a", account, "-w", value])
        .output()
        .map_err(|e| format!("Could not save to the Keychain: {e}"))?;
    o.status.success().then_some(()).ok_or_else(|| format!("Could not save to the Keychain: {}", String::from_utf8_lossy(&o.stderr).trim()))
}

fn kc_del(account: &str) {
    let _ = Command::new("/usr/bin/security").args(["delete-generic-password", "-s", SERVICE, "-a", account]).output();
}

fn saved() -> Option<Saved> {
    kc_get("license").and_then(|s| serde_json::from_str(&s).ok())
}

/// First launch starts the trial.
fn trial_start() -> i64 {
    if let Some(t) = kc_get("trial").and_then(|s| s.parse().ok()) {
        return t;
    }
    let t = now();
    let _ = kc_set("trial", &t.to_string());
    t
}

/// Pure: what the app is allowed to do.
pub fn status_of(trial_start: i64, now: i64, lic: Option<&Saved>) -> Status {
    if let Some(l) = lic {
        let tail: String = l.key.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
        return Status { state: "licensed", days_left: 0, key: tail };
    }
    let used = (now - trial_start).max(0) / 86_400;
    let left = (TRIAL_DAYS - used).max(0);
    Status { state: if left > 0 { "trial" } else { "expired" }, days_left: left, key: String::new() }
}

pub fn current() -> Status {
    match std::env::var("SERVER_HUB_LICENSE").as_deref() {
        Ok("trial") => return Status { state: "trial", days_left: 9, key: String::new() },
        Ok("expired") => return Status { state: "expired", days_left: 0, key: String::new() },
        Ok("licensed") => return Status { state: "licensed", days_left: 0, key: "7F3A".into() },
        _ => {}
    }
    status_of(trial_start(), now(), saved().as_ref())
}

/// Guard runs during the trial and with a key.
pub fn pro() -> bool {
    current().state != "expired"
}

fn post(action: &str, form: &[(&str, &str)]) -> Result<serde_json::Value, String> {
    let mut c = Command::new("/usr/bin/curl");
    c.args(["-sS", "--max-time", "20", "-X", "POST", "-H", "Accept: application/json"]);
    for (k, v) in form {
        c.arg("--data-urlencode").arg(format!("{k}={v}"));
    }
    let o = c.arg(format!("{API}/{action}")).output().map_err(|e| format!("Could not reach Lemon Squeezy: {e}"))?;
    if !o.status.success() {
        return Err("Could not reach Lemon Squeezy — check your internet connection and try again.".into());
    }
    serde_json::from_slice(&o.stdout).map_err(|_| "Lemon Squeezy sent an answer the app could not read. Try again in a minute.".into())
}

/// Pure: reads an activate answer → the instance id, or a message for the user.
pub fn read_activation(v: &serde_json::Value) -> Result<String, String> {
    let product = v.pointer("/meta/product_name").and_then(|p| p.as_str()).unwrap_or("");
    if v["activated"].as_bool() == Some(true) {
        if !product.contains(PRODUCT) {
            return Err(format!("This key is for {}, not {PRODUCT}.", if product.is_empty() { "another product" } else { product }));
        }
        return v.pointer("/instance/id").and_then(|i| i.as_str()).map(String::from).ok_or_else(|| "Lemon Squeezy did not return an activation id. Try again.".into());
    }
    let err = v["error"].as_str().unwrap_or("This key could not be activated.");
    if err.to_lowercase().contains("activation limit") {
        let limit = v.pointer("/license_key/activation_limit").and_then(|n| n.as_i64()).unwrap_or(2);
        return Err(format!("This key is already active on {limit} Macs. Open License on one of them and click Deactivate, then try again here."));
    }
    if err.to_lowercase().contains("not found") {
        return Err("That key was not found. Copy it again from your receipt email, with no spaces.".into());
    }
    Err(err.to_string())
}

/// Shown in the buyer's Lemon Squeezy order next to each activation, so they know which one to free.
fn computer_name() -> String {
    Command::new("/usr/sbin/scutil")
        .args(["--get", "ComputerName"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Mac".into())
}

pub fn activate(key: &str) -> Result<Status, String> {
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err("Paste the license key from your receipt email.".into());
    }
    let v = post("activate", &[("license_key", &key), ("instance_name", &computer_name())])?;
    let instance = read_activation(&v)?;
    let s = Saved { key, instance, checked: now() };
    kc_set("license", &serde_json::to_string(&s).map_err(|e| e.to_string())?)?;
    Ok(current())
}

/// Frees this Mac's activation so the key can go on another one.
pub fn deactivate() -> Result<Status, String> {
    if let Some(s) = saved() {
        let v = post("deactivate", &[("license_key", &s.key), ("instance_id", &s.instance)])?;
        if v["deactivated"].as_bool() != Some(true) && !v["error"].as_str().unwrap_or("").to_lowercase().contains("not found") {
            return Err(v["error"].as_str().unwrap_or("Could not deactivate this Mac. Try again.").to_string());
        }
        kc_del("license");
    }
    Ok(current())
}

/// Once a week: is the key still good (not refunded / disabled)? No internet = keep working.
pub fn recheck() {
    if let Some(mut s) = saved().filter(|s| now() - s.checked > RECHECK_SECS) {
        if let Ok(v) = post("validate", &[("license_key", &s.key), ("instance_id", &s.instance)]) {
            if v["valid"].as_bool() == Some(false) && v.get("license_key").is_some() {
                kc_del("license");
            } else {
                s.checked = now();
                if let Ok(j) = serde_json::to_string(&s) {
                    let _ = kc_set("license", &j);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    const DAY: i64 = 86_400;

    #[test]
    fn trial_counts_down_then_expires() {
        assert_eq!(status_of(0, 0, None), Status { state: "trial", days_left: 14, key: String::new() });
        assert_eq!(status_of(0, 13 * DAY + 5, None).days_left, 1);
        assert_eq!(status_of(0, 14 * DAY, None).state, "expired");
        // clock set back before the trial started: still 14 days, not more
        assert_eq!(status_of(100 * DAY, 0, None).days_left, 14);
    }

    #[test]
    fn a_key_wins_over_the_trial() {
        let s = Saved { key: "AAAA-BBBB-CCCD".into(), instance: "i".into(), checked: 0 };
        assert_eq!(status_of(0, 99 * DAY, Some(&s)), Status { state: "licensed", days_left: 0, key: "CCCD".into() });
    }

    #[test]
    fn reads_activation_answers() {
        let ok = json!({"activated": true, "instance": {"id": "inst-1"}, "meta": {"product_name": "RAM Rescue Pro"}});
        assert_eq!(read_activation(&ok).unwrap(), "inst-1");
        let other = json!({"activated": true, "instance": {"id": "x"}, "meta": {"product_name": "Markdown Everywhere"}});
        assert!(read_activation(&other).unwrap_err().contains("Markdown Everywhere"));
        let full = json!({"activated": false, "error": "This license key has reached the activation limit.", "license_key": {"activation_limit": 2}});
        assert!(read_activation(&full).unwrap_err().contains("already active on 2 Macs"));
        let bad = json!({"activated": false, "error": "license_key not found."});
        assert!(read_activation(&bad).unwrap_err().starts_with("That key was not found"));
    }
}
