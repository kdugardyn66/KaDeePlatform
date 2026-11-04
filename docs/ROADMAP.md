# Roadmap

The KaDee Platform has four layers (see the architecture diagram): native clients in Rust and GPUI, core crates in-process, SDKs, and a Docker stack for collaboration. Modules are ported in the order below; each brings the core crates it needs.

| Version | Module | Source | New crates |
|---|---|---|---|
| **0.1.0** | Start screen and KaDee AI (standalone) | KadeeStudio 0.3.1 | kd-commands, kd-store, kd-ai, kd-ui |
| 0.2 | Markdown editor (MdCraft in a tab) | KaDeeStudio, mdcraft | kd-doc, kd-formats (md), kd-render (text) |
| 0.3 | PDF editor | genoffice, ONLYOFFICE DesktopEditors, pdfcraft | kd-formats (pdf) |
| 0.4 | Drawing / vector | vectorcraft | kd-render (vector) |
| 0.5 | Word processor / DTP | genoffice, ONLYOFFICE, wordcraft | kd-formats (ooxml, odf) |
| 0.6 | Spreadsheet | genoffice, ONLYOFFICE, gridcraft | kd-calc |
| 0.7 | Slide decks | genoffice, ONLYOFFICE, deckcraft | — |
| 0.8 | HTML page processor | genoffice | — |
| 0.9 | Business Intelligence | Rust BI Platform plan | DuckDB, dashboards and reports |
| later | CAD, Design, Effect, Fig, Film, Light, Photo, Solid, Sound, Texture | the Apps (ArtCraft) | kd-plugin (wasmtime) |

Running modes: **Solo, no server** (now) → **Team, docker compose** (kd-sync, kd-collab, kd-api, kd-auth, PostgreSQL, MinIO) → **Organisation, Kubernetes** (Helm chart).

## Decisions in 0.1.0

- **GPUI from crates.io (0.2.2)**, not a git revision of Zed, so builds are reproducible. Component libraries for GPUI (gpui-component) track a newer, separately published GPUI and are left out; kd-ui has the few widgets the start screen needs, including a text input.
- **Recent and Starred in SQLite**, settings in `home.json`: SQLite is the file the offline queue and version history will join.
- **A config folder of its own** (`be.Kadee.KaDeePlatform`), imported once from KadeeStudio 0.x, so the two apps never write each other's files.
- **Every shortcut is a GPUI key binding** generated from kd-commands' key map, so the key map stays toolkit-free while GPUI handles key sequences and text-input precedence.
