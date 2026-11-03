# Changelog

## 0.1.0 — 2026-11-03

The KaDee Platform's first version: KadeeStudio's start screen as a standalone GPUI app.

- **New workspace** in the platform layout: `apps/kadee-studio` and the crates `kd-commands`, `kd-store`, `kd-ai` and `kd-ui`.
- **GPUI** (0.2.2) replaces egui: the whole window is drawn on the GPU (Metal, Vulkan, DirectX).
- **Start screen** ported from KadeeStudio 0.3.1: tab strip, sidebar with the folder tree, greeting, quick-start tiles, Crafting Apps, the documents table with filters, search, sorting, stars and row menus; Recent, Starred, folder pages, New document and Crafting Apps pages; file tabs with details and a preview.
- **KaDee AI** in a pane on the right, from `kadee-ai` (now `kd-ai`): streaming chat with Stop, New chat, model picker, starred files as context, a file's chat in its tab, suggestions, light Markdown, tokens and cost.
- **Settings** in the middle pane: General, Keyboard, AI providers (key, endpoint, Test connection, on/off), AI models (default model), About, licences. Changes apply and are saved at once.
- **MdCraft** joins the Crafting Apps (sixteen in all): it is started from the start screen, and Markdown file tabs offer *Open in MdCraft*.
- **Recent and Starred in SQLite** (`library.sqlite3`); settings stay in `home.json`.
- **Import from KadeeStudio 0.x** on the first launch: settings, Recent, Starred and `ai.toml`.
- **Shortcuts** are GPUI key bindings generated from kd-commands' key map, including sequences (`Cmd+N M`).
- A text input for GPUI in `kd-ui` (one line or wrapped lines, selection, clipboard, IME, masking) for search, names, API keys and the AI composer.

Not yet in 0.1.0 (KadeeStudio 0.3 has them): the Kadee Markdown editor, the shortcut editor, adding custom AI providers and per-module models, and the Done/Cancel of Settings.
