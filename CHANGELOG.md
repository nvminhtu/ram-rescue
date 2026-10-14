# Changelog

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
