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
- Node.js 22+
- PNPM

Install PNPM by following [pnpm.io/installation](https://pnpm.io/installation).

Then install the Wails CLI:

```
go install github.com/wailsapp/wails/v3/cmd/wails3@v3.0.0-beta.5
```

> We use Wails v3 beta (`v3.0.0-beta.5`). Make sure you install the matching CLI version. See [Wails v3 Installation](https://v3.wails.io/getting-started/installation) for details.

### Live Development

Run `wails3 dev` in the project directory. This starts a Vite development server with fast hot reload for frontend changes. You can also connect via http://localhost:34115 in your browser and call your Go methods from devtools.

### Building

To build a production package:

```
wails3 task build:prod
```

### Project Structure

| Directory   | Description                                                 |
|-------------|-------------------------------------------------------------|
| `backend/`  | Go backend (addon logic, API client, services, auth)        |
| `frontend/` | React + TypeScript frontend (Vite, Tailwind CSS, shadcn/ui) |
| `build/`    | Build configuration, icons, installer scripts               |

### Tech Stack

- **Desktop framework:** Wails v3 (Go + WebView2)
- **Frontend:** React, TypeScript, Vite, Tailwind CSS v4, shadcn/ui
- **State management:** Zustand + Jotai
- **Backend:** Go, Viper (config), Zap (logging)
- **Packaging:** NSIS (Windows), nfpm (Linux)
