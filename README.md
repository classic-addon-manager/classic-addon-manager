# Classic Addon Manager

A desktop application for managing addons in **ArcheAge**. Browse, install, update, and troubleshoot your addons all from one place.

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/gaijindev)

<img align="center" width="700" alt="Dashboard screenshot" src="https://github.com/user-attachments/assets/4a5d13a8-dc4d-481e-85a5-5fdc0ce4f98b" />

<img align="center" width="700" alt="Addon browser screenshot" src="https://github.com/user-attachments/assets/c1ee5165-ff20-4717-810f-d4b065945521" />

<img align="center" width="700" alt="Troubleshooting screenshot" src="https://github.com/user-attachments/assets/a1bc8d7f-b2de-4b0e-bed8-ac4eeeacd683" />

<img align="center" width="700" alt="Settings screenshot" src="https://github.com/user-attachments/assets/5e9906bf-667b-4242-930a-d04af6d21f34" />

---

## Features

- **Addon Browser**: Browse the addon repository, filter by tags, and install addons with one click. Dependencies are resolved automatically.
- **Dashboard**: See all your installed addons at a glance. Check for updates, uninstall, open addon folders, or report issues.
- **In-Game Notifications**: Get notified inside ArcheAge when addon updates are available.
- **AI Chat Assistant**: Ask a friendly Daru for help with addons, game questions, or troubleshooting.
- **Troubleshooting Tools**: Diagnose addon issues by scanning your game logs for errors, reset corrupted settings, or uninstall everything and start fresh.
- **Self-Updating**: The app checks for its own updates and can install them automatically.
- **Discord Authentication**: Sign in with Discord to sync your installed addons, rate addons, and access the AI assistant.
- **ZIP Import**: Install addons from a local ZIP file.

---

## Download

You can get Classic Addon Manager in one of these ways:

- **ArcheAge launcher**: Click the **Addons** button in the launcher.
- **GitHub Releases**: Download the latest `classic-addon-manager.exe` from the [Releases page](https://github.com/classic-addon-manager/classic-addon-manager/releases).
- **In-app self-update**: The app notifies you when a new version is available and can update itself.

### Supported Platforms

- **Windows**: Full support (.exe installer)
- **Linux**: Supported (.deb package), but designed primarily for Windows. Some features may not work

---

## Support

If you find this project useful, consider supporting development:

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/X8X219OKGE)

---

## Development

### Requirements

- Go 1.25+
- Node.js 22.12+
- PNPM

Install PNPM by following [pnpm.io/installation](https://pnpm.io/installation).

Then install the Wails CLI:

```
go install github.com/wailsapp/wails/v3/cmd/wails3@v3.0.0-beta.5
```

> We use Wails v3 beta (`v3.0.0-beta.5`). Make sure you install the matching CLI version. See [Wails v3 Installation](https://v3.wails.io/getting-started/installation) for details.

### Live Development

Run `wails3 dev` in the project directory. This starts a Vite development server with fast hot reload for frontend changes, plus the Wails window. Inspect the interface in the app window itself: development builds enable the WebView2 DevTools, so right-click → **Inspect** (or `F12` where the host page allows it) gives you the full app with working bindings.

Press **Ctrl+Alt+Q** while the app is focused to open or close **TanStack Query Devtools** and inspect cached queries and mutations. This shortcut is available only in frontend development mode.

The Vite URL (`http://localhost:9245`) is the frontend dev server only. It does not serve the Wails runtime: `/wails/runtime` and the other `/wails/*` endpoints are answered by the app's asset server, which is reachable inside the WebView (`http://wails.localhost:<vite-port>`) and not over a TCP port. Opening the Vite URL in a normal browser therefore has no Go bindings or events, and this app renders blank because it bootstraps from them. Use it only for isolated frontend work that does not touch the backend.

For a browser with working bindings and events, run the app in server mode:

```
wails3 task run:server
```

Then open http://localhost:8080. Server mode serves the same frontend and all service bindings over HTTP, with events delivered over WebSocket.

### Frontend Linting and Formatting

Run these commands from `frontend/` after `pnpm install`:

| Command             | Purpose                                 |
|---------------------|-----------------------------------------|
| `pnpm lint`         | Run oxlint and apply safe fixes         |
| `pnpm lint:check`   | Run oxlint without changing files       |
| `pnpm format`       | Format TypeScript/TSX with Prettier     |
| `pnpm format:check` | Check formatting without changing files |

Oxlint is configured in `frontend/.oxlintrc.json`. It uses native core, TypeScript,
React Hooks, Fast Refresh, and React Compiler rules.

Formatting is separate from linting and retains `frontend/.prettierrc`. Generated
Wails bindings and build output are excluded from both checks. Run `pnpm lint`
before `pnpm format`, overlapping type-import and import-sorting fixes can require
a second lint run.

### Building

To build a production package:

```
wails3 task build:prod
```

To build the server binary (no GUI window, serves the frontend and all bindings over HTTP):

```
wails3 task build:server
```

The binary is written to `build/bin/classic-addon-manager-server` (`.exe` on Windows). It defaults to http://localhost:8080; override with `WAILS_SERVER_HOST` / `WAILS_SERVER_PORT`. Pass `PRODUCTION=true` for a stripped production build (`-tags server,production`).

### Version Management

Use the `version` task instead of editing version numbers by hand. It keeps `build/config.yml`, the Windows resource, manifest and NSIS files, the Linux package config, and `backend/shared/types.go` in sync.

```
# Set an explicit version
wails3 task version SET=3.3.0

# Bump to the next version (defaults to patch)
wails3 task version BUMP=major
wails3 task version BUMP=minor
wails3 task version BUMP=patch
```

With no arguments, the task bumps the patch version. It changes only version fields and leaves other build settings alone.

- `BUMP` requires all version fields to match. If they differ or contain an invalid version, the task stops without writing any files.
- `SET` accepts `X.Y.Z` or `vX.Y.Z` and updates all version fields, even if their current values differ or are invalid.
- Run the task from the project root. You can also run the tool directly with `go run ./tools/setversion -set 3.3.0` or `go run ./tools/setversion -bump patch`.

### Project Structure

| Directory   | Description                                                 |
|-------------|-------------------------------------------------------------|
| `backend/`  | Go backend (addon logic, API client, services, auth)        |
| `frontend/` | React + TypeScript frontend (Vite, Tailwind CSS, shadcn/ui) |
| `build/`    | Build configuration, icons, installer scripts               |

### Tech Stack

- **Desktop framework:** Wails v3 (Go + WebView2)
- **Frontend:** React, TypeScript, Vite, Tailwind CSS v4, shadcn/ui
- **Fonts:** Geist + Geist Mono
- **State management:** Zustand + Jotai
- **Backend:** Go, Viper (config), Zap (logging)
- **Packaging:** NSIS (Windows), nfpm (Linux)
