//! Guard (Pro): watches memory in the background. Pure rules here, fed with samples and a clock, so they are testable;
//! the desktop shell calls `Guard::tick` once a minute (leaks, orphans) and `Guard::pressure` every few seconds.
use std::collections::{HashMap, VecDeque};

/// Samples kept per app group (one a minute → 90 minutes).
const KEEP: usize = 90;
/// A leak needs this much history, …
const MIN_SPAN_SECS: u64 = 45 * 60;
/// … at least this much memory now, …
const MIN_MB: u64 = 1024;
/// … growth over the whole window (MB per hour), …
const LEAK_MB_PER_H: f64 = 1024.0;
/// … still growing over the last 20 minutes (half the rate is enough), …
const RECENT_SECS: u64 = 20 * 60;
/// … and never fell more than 20 % below its running peak (garbage collection = not a leak).
const MAX_DROP: f64 = 0.20;
/// Pressure must stay warning/critical this long before we warn.
const PRESSURE_HOLD_SECS: u64 = 30;
/// Do not warn about the same app twice within this long.
const COOLDOWN_SECS: u64 = 30 * 60;

/// Least-squares slope of (t, mb) in MB per hour.
fn slope_mb_per_h(pts: &[(u64, u64)]) -> f64 {
    let n = pts.len() as f64;
    if pts.len() < 2 {
        return 0.0;
    }
    let mt = pts.iter().map(|p| p.0 as f64).sum::<f64>() / n;
    let mm = pts.iter().map(|p| p.1 as f64).sum::<f64>() / n;
    let (mut num, mut den) = (0.0, 0.0);
    for &(t, m) in pts {
        num += (t as f64 - mt) * (m as f64 - mm);
        den += (t as f64 - mt).powi(2);
    }
    if den == 0.0 { 0.0 } else { num / den * 3600.0 }
}

/// Growth in MB per hour when the samples look like a leak (steady, still going, never released), else None.
pub fn leak_rate(pts: &[(u64, u64)]) -> Option<f64> {
    let (first, last) = (pts.first()?, pts.last()?);
    if last.0 - first.0 < MIN_SPAN_SECS || last.1 < MIN_MB {
        return None;
    }
    let mut peak = 0u64;
    for &(_, m) in pts {
        peak = peak.max(m);
        if (m as f64) < peak as f64 * (1.0 - MAX_DROP) {
            return None;
        }
    }
    let rate = slope_mb_per_h(pts);
    let recent: Vec<(u64, u64)> = pts.iter().copied().filter(|p| p.0 + RECENT_SECS >= last.0).collect();
    (rate >= LEAK_MB_PER_H && slope_mb_per_h(&recent) >= LEAK_MB_PER_H / 2.0).then_some(rate)
}

/// What Guard wants the shell to do.
#[derive(Debug, PartialEq)]
pub enum Alert {
    /// Memory pressure is high: name the heaviest app the user may quit.
    Pressure { id: String, name: String, mb: u64 },
    Leak { id: String, name: String, mb: u64, mb_per_h: u64 },
}

/// One row of the memory table, as Guard needs it.
pub struct Row<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub mb: u64,
    pub quittable: bool,
    pub orphan: bool,
}

#[derive(Default)]
pub struct Guard {
    samples: HashMap<String, VecDeque<(u64, u64)>>,
    high_since: Option<u64>,
    warned: HashMap<String, u64>,
    orphan_seen: HashMap<String, u64>,
    /// Groups currently flagged as leaking → MB per hour (shown as "Leaking +X GB/h").
    pub leaking: HashMap<String, u64>,
}

impl Guard {
    fn fresh(&mut self, key: String, now: u64) -> bool {
        match self.warned.get(&key) {
            Some(&t) if now < t + COOLDOWN_SECS => false,
            _ => {
                self.warned.insert(key, now);
                true
            }
        }
    }

    /// Every few seconds: warn once pressure has stayed high for 30 s, naming the heaviest quittable app.
    pub fn pressure(&mut self, now: u64, level: &str, rows: &[Row]) -> Option<Alert> {
        if level == "normal" {
            self.high_since = None;
            return None;
        }
        let since = *self.high_since.get_or_insert(now);
        if now < since + PRESSURE_HOLD_SECS {
            return None;
        }
        let top = rows.iter().filter(|r| r.quittable).max_by_key(|r| r.mb)?;
        self.fresh(format!("p:{}", top.id), now)
            .then(|| Alert::Pressure { id: top.id.into(), name: top.name.into(), mb: top.mb })
    }

    /// Once a minute: record samples, refresh `leaking`, return new leak alerts.
    pub fn tick(&mut self, now: u64, rows: &[Row]) -> Vec<Alert> {
        let live: std::collections::HashSet<&str> = rows.iter().map(|r| r.id).collect();
        self.samples.retain(|id, _| live.contains(id.as_str()));
        self.leaking.clear();
        let mut out = vec![];
        for r in rows.iter().filter(|r| r.quittable) {
            let s = self.samples.entry(r.id.to_string()).or_default();
            s.push_back((now, r.mb));
            while s.len() > KEEP {
                s.pop_front();
            }
            let pts: Vec<(u64, u64)> = s.iter().copied().collect();
            if let Some(rate) = leak_rate(&pts) {
                self.leaking.insert(r.id.to_string(), rate as u64);
                if self.fresh(format!("l:{}", r.id), now) {
                    out.push(Alert::Leak { id: r.id.into(), name: r.name.into(), mb: r.mb, mb_per_h: rate as u64 });
                }
            }
        }
        out
    }

    /// Orphans first seen more than `after_secs` ago (ids to quit). Forgets orphans that are gone.
    pub fn stale_orphans(&mut self, now: u64, rows: &[Row], after_secs: u64) -> Vec<String> {
        let ids: Vec<&str> = rows.iter().filter(|r| r.orphan && r.quittable).map(|r| r.id).collect();
        self.orphan_seen.retain(|id, _| ids.contains(&id.as_str()));
        ids.into_iter()
            .filter(|id| *self.orphan_seen.entry(id.to_string()).or_insert(now) + after_secs <= now)
            .map(str::to_string)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One sample a minute for `mins` minutes, memory from `f(minute)`.
    fn series(mins: u64, f: impl Fn(u64) -> u64) -> Vec<(u64, u64)> {
        (0..mins).map(|m| (m * 60, f(m))).collect()
    }

    #[test]
    fn steady_growth_is_a_leak() {
        // 1.2 GB → +25 MB a minute (1.5 GB/h) for an hour
        let r = leak_rate(&series(60, |m| 1200 + m * 25)).expect("leak");
        assert!((1400.0..1600.0).contains(&r), "{r}");
    }

    #[test]
    fn jump_then_flat_is_not_a_leak() {
        let pts = series(60, |m| if m < 30 { 1100 } else { 3000 });
        assert_eq!(leak_rate(&pts), None);
    }

    #[test]
    fn sawtooth_garbage_collection_is_not_a_leak() {
        // grows 40 MB/min, drops back every 15 minutes
        assert_eq!(leak_rate(&series(60, |m| 1200 + (m % 15) * 40 + m * 10)), None);
    }

    #[test]
    fn small_or_short_is_not_a_leak() {
        assert_eq!(leak_rate(&series(60, |m| 200 + m * 10)), None, "too small");
        assert_eq!(leak_rate(&series(30, |m| 1200 + m * 30)), None, "too short");
        assert_eq!(leak_rate(&series(60, |m| 2000 + m)), None, "too slow");
    }

    fn row<'a>(id: &'a str, mb: u64) -> Row<'a> {
        Row { id, name: id, mb, quittable: true, orphan: false }
    }

    #[test]
    fn pressure_waits_30s_names_the_top_app_and_does_not_repeat() {
        let mut g = Guard::default();
        let rows = [row("chrome", 6000), row("docker", 2000), Row { id: "kernel", name: "kernel", mb: 9000, quittable: false, orphan: false }];
        assert_eq!(g.pressure(0, "warning", &rows), None);
        assert_eq!(g.pressure(31, "warning", &rows), Some(Alert::Pressure { id: "chrome".into(), name: "chrome".into(), mb: 6000 }));
        assert_eq!(g.pressure(60, "critical", &rows), None, "cooldown");
        assert_eq!(g.pressure(61, "normal", &rows), None);
        assert_eq!(g.pressure(31 * 60 + 100, "warning", &rows), None, "hold again after normal");
        assert!(g.pressure(31 * 60 + 131, "warning", &rows).is_some(), "after cooldown");
    }

    #[test]
    fn tick_flags_and_alerts_a_leak_once() {
        let mut g = Guard::default();
        let mut alerts = 0;
        for m in 0..70u64 {
            let rows = [row("chrome", 1200 + m * 25), row("slack", 800)];
            alerts += g.tick(m * 60, &rows).len();
        }
        assert_eq!(alerts, 1);
        assert!(g.leaking.contains_key("chrome") && !g.leaking.contains_key("slack"));
    }

    #[test]
    fn orphans_are_stale_after_the_delay_and_forgotten_when_gone() {
        let mut g = Guard::default();
        let o = [Row { id: "p:42", name: "vite", mb: 300, quittable: true, orphan: true }, row("chrome", 5000)];
        assert!(g.stale_orphans(0, &o, 1800).is_empty());
        assert_eq!(g.stale_orphans(1800, &o, 1800), vec!["p:42".to_string()]);
        assert!(g.stale_orphans(1900, &o[1..], 1800).is_empty());
        assert!(g.stale_orphans(2000, &o, 1800).is_empty(), "came back = new orphan, timer restarts");
    }
}
