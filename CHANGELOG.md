# Changelog

## 0.6.0 — Clean, History, Meeting mode

- **Clean (free).** A new tab lists caches that apps rebuild by themselves — Xcode build files and iOS device support,
  simulator caches, npm, Yarn, Gradle, CocoaPods, Homebrew, pip, Playwright browsers, and the caches of Chrome, Edge,
  Brave, Arc, Firefox and Safari — with their real size. Tick what you want and they go to the **Trash** (nothing is
  deleted until you empty it). Browser caches only while that browser is closed. Never cookies, history, passwords or
  your files: you stay signed in everywhere.
- **History (Pro).** The last 24 hours of memory, by app: see what slowed your Mac down, the peak and the heaviest app.
- **Meeting mode (Pro).** Pick the heavy apps once; one click quits them before a call or a game, one click opens them
  again. Also in the menu bar.
- **Quit when not used (Pro).** Rules like "quit Docker after 30 minutes in the background, when memory is tight".
- Guard → Recent also lists what idle rules and Meeting mode did.

## 0.5.0 — RAM Rescue Pro

- **New look.** A big memory ring on the left, the heaviest apps as cards with their real icons and a large Quit
  button, orphans in their own card. The full process list is one click away ("All processes"). Light and dark.
- **New icon.** A lifebuoy around a memory stick.
- **Guard (Pro).** RAM Rescue now watches memory from the menu bar, even with the window closed:
  - **Warns before your Mac slows down** — when memory pressure stays yellow or red, a notification names the app
    using the most memory, with a **Quit** button.
  - **Catches apps that leak memory** — an app that keeps growing (about 1 GB an hour or more, for 45 minutes, without
    ever giving memory back) gets a red **Leaking** tag and one notification.
  - **Auto-quits orphans** (off by default) — dev processes left behind by a closed terminal, editor or agent are quit
    after 15 / 30 / 60 minutes. Everything Guard does is listed under Guard → Recent.
- **Menu bar shows memory.** RAM in use (⚠︎ when pressure is high) and the 3 heaviest apps with Quit, for everyone.
- **Pricing.** Everything you had stays free. Guard is Pro: $9.99 once, 2 Macs, 14-day free trial — when it ends,
  Guard turns off and the rest keeps working.

## 0.4.0 — RAM Rescue

- **New name: RAM Rescue** (was Server Hub). The app is now first of all a memory rescuer; settings carry over.
- **Memory tab first.** Opens on it by default.
- **Heavy apps are big cards** (500 MB and up): RAM in large type, % of your RAM, a big red **Quit**, Force Quit in the ⌄
  menu, and **Free X GB** to quit every heavy app at once (asks first).
- **Stop / Quit buttons you can see.** Every row in Memory, Localhost and Projects has a labelled red Stop/Quit button, no
  hovering needed. Stop still asks a second click ("Stop?").
- **iOS Simulator is one row.** Its 150+ background daemons (routined, pasted, photoanalysisd…) count as one app.

## 0.3.0

- **Memory tab.** Every app and background process by RAM, apps grouped with their helpers (`Google Chrome · 45`),
  a RAM bar with memory pressure, filters **High / Medium / Low / All**, **Hide system** on by default. Quit asks first
  and shows how much it frees; Force Quit for apps that hang; tick several rows to quit them together.
- **Orphans.** Dev processes left behind when their terminal, editor or agent closed (vite, tsc --watch, Playwright
  Chromium, claude/codex CLIs). Own filter with **Left by** and **Quit all**, a red count on the Memory tab, and an
  `orphan` tag on the Localhost tab.
- **Native macOS table look.** Column headers (Port · Name · Mem · Up), compact striped rows, blue selection; healthy rows
  show no text, only problems (HTTP 500) in red. Row buttons appear only on hover or for the selected row.
- **Less clutter.** Detail panel keeps one Open button, a … menu (Share on LAN, Show in Finder, Copy) and Stop; the log is
  folded behind **▸ Log** until you open it. Projects no longer repeat a folder name above its single project.
- **Dark mode** follows macOS.

## 0.2.2

- **Plain macOS look.** White window, light gray toolbar and sidebar, system font and colors — no more dark terminal theme.
- **Smaller window.** Opens at 900×560 and shrinks down to 640×400; the toolbar is shorter and never cuts off buttons.
- **Short rows, details on click.** A row shows just status, port, name, open and start/stop. Click it and a panel opens
  below with the URL, health, LAN link, command, folder, uptime, memory, PID and who started it, plus Open, Share on LAN,
  Show in Finder, Copy and Stop, and the log. Esc or ✕ closes it.

## 0.2.1

- **Sidebar starts closed.** More room for the list; open it with ☰ or ⌘B. New **Localhost / Projects** tabs in the top bar.
- **Bigger ▶ / ■ buttons**, always visible, green to start and red to stop.
- Back from 0.1.2: projects sitting straight in a folder group under that folder (e.g. **CODE**), and a running project
  that answers HTTP 5xx gets a red dot in the Projects list.
- The top bar no longer wraps on narrower windows.

## 0.2.0

- **Running servers only by default.** The project list and the sidebar show just what is running; tick **Show all**
  to see every project, stopped ones included. Server Hub remembers your choice.
- **Share on your local network (⇄).** Open a dev server from your phone or another computer on the same Wi-Fi.
  Servers that already listen on every address get their LAN link right away; servers bound to `127.0.0.1` only
  (vite, next dev…) get a small relay on `<port + 10000>`, e.g. `5173 → http://192.168.1.20:15173`. The link is copied
  for you; click ⇄ twice to stop sharing. Relays close when the server stops or you quit Server Hub.
- **Open source** under the MIT license.

## 0.1.1

- Drag the window from the top bar; scan several project folders.

## 0.1.0

- First public release: Localhost view with health checks, Projects view with ▶ / ■ and live logs, menu bar icon.
