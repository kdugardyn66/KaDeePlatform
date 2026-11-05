# Roadmap

The KaDee Platform has four layers (see the architecture diagram): native clients in Rust and GPUI, core crates in-process, SDKs, and a Docker stack for collaboration. Modules are ported in the order below; each brings the core crates it needs.

| Version | Module | Source | New crates |
|---|---|---|---|
| **0.1.0** | Start screen and KaDee AI (standalone) | KadeeStudio 0.3.1 | kd-commands, kd-store, kd-ai, kd-ui |
| **0.2** (in progress) | Kadee Markdown: MdCraft rebuilt in GPUI, in a tab | KaDeeStudio, mdcraft | kd-doc, kd-formats (md), kd-markdown |
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

## Decisions in 0.2

- **MdCraft is rebuilt natively, not embedded.** MdCraft draws its interface in a web view; Kadee Markdown draws the same ribbon, editor and preview with GPUI, so it fits the platform and needs no browser engine.
- **The ribbon is data** (`kd-markdown/src/ribbon.rs`) and every button runs a command by MdCraft's id (`commands.rs`), so shortcuts, command search and later the SDK and agents share them.
- **Pure logic in kd-formats and kd-doc** (no GPUI): parsing, editing operations and highlighting are tested on their own.
- **MdCraft's AI layer and MCP server are not carried over.** KaDee AI is the one AI layer; it reads the editor's text and inserts answers through the same edit path as typing (one undo step).
- **Diagrams as SVG, drawn by kd-formats** (no browser, no JavaScript): the same SVG goes into the preview (rasterised with resvg at 2×) and into exported HTML. GPUI 0.2's own SVG images come out with red and blue swapped, so kd-markdown rasterises them itself.
- **Print and PDF through the browser** for now: the exported page carries the page setup (`@page`), so the browser's *Save as PDF* gives the paper size and margins of the front matter.
- **Templates are files**: `.mdd` and `.mds` in the templates folder (KaDee's own and MdCraft's are both listed), so they can be shared and edited like any document.
- **Next**: spelling, typeset math, the remaining Mermaid kinds and Graphviz, native PDF export, running code chunks.
