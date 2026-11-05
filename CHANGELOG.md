# Changelog

## 0.2.0 — in progress

Kadee Markdown: MdCraft rebuilt natively in GPUI and opened in KadeeStudio's tabs. This first step brings MdCraft's ribbon, editor, preview, navigation pane and status bar; the remaining ribbon commands follow.

- **`.md` files open in Kadee Markdown** inside their tab (instead of the details page): MdCraft's layout with the **File** tab, **Home, Insert, Design, Slides, Review, View** and the contextual **Table** tab, command search (*Search commands*), the document name and quick buttons (Save, Undo, Redo, KaDee AI, collapse the ribbon).
- **The ribbon as in MdCraft**: the same groups, large / small / icon buttons, split buttons with their menus (Diagram, Chart, Callout, Code block, Equation, Present), drop-down buttons (Table with the 8 × 10 grid, Media, Chemistry, Date & time, Symbol, page Size, Orientation, Margins, Watermark), the Styles, document-theme, slide-theme and layout galleries, the font boxes and MdCraft's own two-tone icons. Buttons show their state (bold at the cursor, current heading, view, pane …) and run commands by MdCraft's ids, labels and shortcuts.
- **Commands that work now**: undo/redo, clipboard, select all; bold, italic, underline, strikethrough, inline code, highlight, sub/superscript, font colour, clear formatting; Normal and Heading 1–6; bullets, numbering, checklist, quote, indent; horizontal line, page break, table of contents, callouts, code blocks, equations, inline math, the 15 diagrams and 6 charts, tables (grid), add row / column, delete row, tidy table, footnote, link, picture, collapsible section, date & time, symbols, chemical structure, properties; document theme, paper size, orientation, margins, watermark, numbered headings, justify; new slide with a layout; word count, outline; Write / Split / Read, navigation pane, line numbers, word wrap, scroll with the cursor, zoom, dark page, collapse the ribbon, focus mode, light / dark / system. Commands still to come say so in a message.
- **Source editor** drawn with GPUI: Markdown colours (headings, markup dimmed, bold, italic, code, links, quotes, HTML, comments, front matter, math), word wrap, line numbers, active line, selection with mouse and keys, double and triple click, IME, clipboard, undo/redo with grouped typing, list and quote continuation on Enter, Tab / Shift+Tab indent.
- **Native preview** with MdCraft's GitHub-like document styles in light and dark: headings, inline styles, links (open in the browser; `#anchors` jump), lists and task lists, quotes, callouts, code, tables with alignment, pictures (local files), rules, page and slide breaks, `[TOC]`, footnotes and front matter. Diagrams, charts and math show their source until they are drawn natively. Double-click a block to go to its source line.
- **Navigation pane** with Headings (filter, current heading) and Slides; **status bar** with words, line and column, slides, save state, the views and zoom.
- **Saving**: `Cmd+S` / `Ctrl+S` and Save As write the file safely (temporary file, then rename); unsaved tabs show •, and closing one asks first.
- **KaDee AI in the editor**: MdCraft's AI layer is gone. KaDee AI answers about the document as it is on screen (unsaved text included, and the selection when there is one), and every answer has **Insert into document**.
- New crates: **kd-doc** (text, selection, grouped undo), **kd-formats** (Markdown: parsing with comrak in MdCraft's dialect, editing operations, highlighting, MdCraft's libraries of diagrams, charts, callouts, layouts and themes) and **kd-markdown** (the editor, preview, ribbon and commands).
- Documents named on the command line open at start.

**The rest of the Markdown editor**

- **File types**: `.md` and `.markdown` documents; **templates** `.mdd` (Markdown document template) and `.mds` (Markdown slide deck template). A template opens for editing with a bar offering **New document from this template**; File › **Save as template…** saves the document into the templates folder as `.mdd`, or `.mds` when it has slides.
- **New page › templates**: MdCraft's 11 document and 8 slide deck templates, and **My templates** (from KaDee's templates folder and MdCraft's) with Use and Edit, *New document template*, *New slide deck template* and *Show templates folder*. Placeholders (`{{author}}`, `{{date}}`, `{{date_long}}`, `{{month_year}}`, `{{year}}`, `{{next_year}}`, `{{quarter}}`) are filled in; the author is the name from Settings.
- **Find and Replace** (`Ctrl+F` / `Cmd+F`): matches highlighted in the editor, *n of m*, previous / next, match case, whole words, Replace and Replace all (one undo step). **Go to line** (`Ctrl+G`).
- **Fonts**: the font box lists the installed fonts (type to filter), the size box MdCraft's sizes, grow / shrink, font colour and highlight colours — written as `<span style>` like MdCraft, and shown in the preview. Design › Document fonts set `mainfont`, `monofont` and `fontsize`.
- **Slides**: layout gallery, duplicate, delete, move up / down, speaker notes, background colour, Convert to slides; the preview shows each slide as a 16:9 frame in the deck theme's colours with its notes; the status bar shows *Slide n of m*.
- **Slide show** (`F5`, `Shift+F5`): full screen; arrows, space, Page Up / Down, Home / End, clicks; Esc ends at the slide you were on.
- **Charts and diagrams drawn natively**: Chart.js bar, line, pie, doughnut and radar charts, charts from CSV tables, simple Vega-Lite charts, and Mermaid flowcharts, pie charts, sequence diagrams and Gantt charts. Math shows as Unicode (`\frac`, roots, Greek, super- and subscripts, `\ce{}`). Other diagram kinds still show their source.
- **Insert**: cover page, link (asks for the address), table by size, table from a CSV file, video or audio, online video, include a file (`@import`, shown in the preview), emoji, 3D molecule; custom watermark.
- **Print and Export as PDF** through the browser (the page opens with the print window; *Save as PDF* there), **Export as web page (HTML)** with MdCraft's styles, the theme, diagrams as SVG and page setup, and **Copy as formatted text** (HTML).
- **Review › Check accessibility** (skipped heading levels, pictures without alternative text, vague link text, tables without headers; click an issue to go to it); **Help › Keyboard shortcuts**; **Markdown guide** (the Welcome template as a new document).
- **View**: drag the divider between editor and preview (double-click resets), Page width.
- Not yet: spelling, KaTeX/MathJax-quality math, running code chunks, the remaining Mermaid diagram kinds (class, state, ER, mind map, timeline, journey, quadrant, git graph), Graphviz, PlantUML and WaveDrom.

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
