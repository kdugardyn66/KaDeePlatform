# Changelog

## 0.1.0 — 2026-11-03

The KaDee Platform's first version: KadeeStudio's start screen as a standalone GPUI app.

- **New workspace** in the platform layout: `apps/kadee-studio` and the crates `kd-commands`, `kd-store`, `kd-ai` and `kd-ui`.
- **GPUI** (0.2.2) replaces egui: the whole window is drawn on the GPU (Metal, Vulkan, DirectX).
- **Start screen** ported from KadeeStudio 0.3.1: tab strip, sidebar with the folder tree, greeting, quick-start tiles, Apps, the documents table with filters, search, sorting, stars and row menus; Recent, Starred, folder pages, New document and Apps pages; file tabs with details and a preview.
- **KaDee AI** in a pane on the right, from `kadee-ai` (now `kd-ai`): streaming chat with Stop, New chat, model picker, starred files as context, a file's chat in its tab, suggestions, light Markdown, tokens and cost.
- **Settings** as in KadeeStudio 0.3.1: a Settings card over the middle pane with the sections on the left and **Cancel / Done** at the bottom. Changes show at once; Done (or going to another page) keeps them, Cancel, × or Esc puts back what was there; AI settings are saved on Done.
  - **General**: theme, name in the greeting, *Save new documents in*, *Open each module / file in a new Tab* (Window comes later), team server (Phase 3).
  - **Keyboard**: the shortcut editor — **Change** / **Record** takes the next keys (sequences up to 3, Enter saves, Esc cancels, Backspace starts over, saved after 1.5 s), **Clear**, **Remove**, conflicts shown, **Add a shortcut** (any action, several per action) and **Reset to defaults**.
  - **AI providers**: the providers by region with their status, **Add provider** (choose the service, then its key, endpoint, default model, models; Test connection), and each provider's page with **Remove provider**. Custom providers choose their API and region.
  - **AI models**: the default model, a model per part of KadeeStudio, and the models you added (Remove).
  - About (with the build) and the MIT and Apache-2.0 licences.
- **MdCraft** joins the Apps (sixteen in all): it is started from the start screen, and Markdown file tabs offer *Open in MdCraft*.
- **Recent and Starred in SQLite** (`library.sqlite3`); settings stay in `home.json`.
- **Import from KadeeStudio 0.x** on the first launch: settings, Recent, Starred and `ai.toml`.
- **Shortcuts** are GPUI key bindings generated from kd-commands' key map, including sequences (`Cmd+N M`).
- **Build scripts** as in KadeeStudio: `build.sh`, `scripts/build-macos.sh` (.app, .dmg, universal, signing, notarization; runtime shaders when the Metal compiler is missing), `scripts/build-linux.sh` (.tar.gz, .deb, .rpm, AppImage), `scripts/build-windows.ps1` (.zip, .msi), and the CI and Release workflows. Installers are named KaDee Platform so they sit next to KadeeStudio 0.x.
- Settings › About shows the build (commit and date); the Windows executable has the app icon and runs without a console.
- **Crafting Apps are now called Apps**, and the Apps page shows them as cards in a grid (icon, name, purpose, maker and file types, *Installed · version*), as in KadeeStudio 0.3.1, with the ArtCraft Launcher and *Look again* below.
- A text input for GPUI in `kd-ui` (one line or wrapped lines, selection, clipboard, IME, masking) for search, names, API keys and the AI composer.

Not yet in 0.1.0 (KadeeStudio 0.3 has them): the Kadee Markdown editor and the *Add a model* form (prices and limits for a model the catalogue does not list).
