//! Kadee Markdown's commands: MdCraft's, by the same ids, labels, icons and shortcuts. The
//! ribbon, the shortcuts and command search all run them by id. MdCraft's AI commands are gone;
//! `ai.ask` opens KaDee AI instead.

use gpui::{Action, App, KeyBinding};

/// Run a command by id (bound to the shortcuts).
#[derive(Clone, PartialEq, Debug, Action)]
#[action(namespace = kd_markdown, no_json)]
pub struct RunMd {
    pub id: &'static str,
}

pub const CONTEXT: &str = "KdMarkdown";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cmd {
    pub id: &'static str,
    pub label: &'static str,
    /// MdCraft icon name, or an asset path (`icons/…`) for KaDee's own.
    pub icon: &'static str,
    /// MdCraft notation: `Mod+Shift+S`.
    pub key: Option<&'static str>,
}

const fn c(id: &'static str, label: &'static str, icon: &'static str, key: Option<&'static str>) -> Cmd {
    Cmd { id, label, icon, key }
}

pub const COMMANDS: &[Cmd] = &[
    c("file.backstage", "File", "file", Some("Alt+F")),
    c("file.new", "New…", "file-plus", Some("Mod+N")),
    c("file.newBlank", "New blank document", "file-md", None),
    c("file.newDeck", "New blank presentation", "file-slides", None),
    c("file.open", "Open…", "folder-open", Some("Mod+O")),
    c("file.save", "Save", "save", Some("Mod+S")),
    c("file.saveAs", "Save As…", "save-as", Some("Mod+Shift+S")),
    c("file.saveTemplate", "Save as template…", "save-as", None),
    c("file.saveAsFormat", "Save As", "save-as", None),
    c("file.recent", "Open Recent", "clock", None),
    c("file.clearRecent", "Clear Recent List", "x", None),
    c("file.export", "Export", "export", None),
    c("template.use", "New document from this template", "file-plus", None),
    c("file.print", "Print…", "printer", Some("Mod+P")),
    c("file.exportHtml", "Export as web page (HTML)…", "web", None),
    c("file.exportPdf", "Export as PDF…", "pdf", None),
    c("file.copyRich", "Copy as formatted text", "clipboard-rich", None),
    c("file.close", "Close", "x", None),
    c("file.info", "Document info", "info", None),
    c("file.properties", "Document properties…", "front-matter", None),
    c("file.options", "Options", "settings", Some("Mod+,")),
    c("edit.undo", "Undo", "undo", Some("Mod+Z")),
    c("edit.redo", "Redo", "redo", None),
    c("edit.cut", "Cut", "cut", None),
    c("edit.copy", "Copy", "copy", None),
    c("edit.paste", "Paste", "paste", None),
    c("edit.selectAll", "Select all", "select-all", None),
    c("edit.find", "Find & Replace", "find", Some("Mod+F")),
    c("edit.replace", "Replace…", "replace", None),
    c("edit.goto", "Go to line…", "arrow-left", Some("Mod+G")),
    c("format.bold", "Bold", "bold", Some("Mod+B")),
    c("format.italic", "Italic", "italic", Some("Mod+I")),
    c("format.underline", "Underline", "underline", Some("Mod+U")),
    c("format.strike", "Strikethrough", "strike", Some("Mod+Shift+X")),
    c("format.code", "Inline code", "code", Some("Mod+E")),
    c("format.highlight", "Highlight", "highlight", Some("Mod+Shift+H")),
    c("format.color", "Font color", "font-color", Some("Mod+Shift+C")),
    c("format.highlightColor", "Highlight color", "highlight", None),
    c("format.fontFamily", "Font", "font", None),
    c("format.fontSize", "Font size", "font-size", None),
    c("format.grow", "Increase font size", "font-grow", None),
    c("format.shrink", "Decrease font size", "font-shrink", None),
    c("format.sub", "Subscript", "subscript", Some("Mod+=")),
    c("format.sup", "Superscript", "superscript", Some("Mod+Shift+=")),
    c("format.clear", "Clear formatting", "clear", Some("Mod+Space")),
    c("format.normal", "Normal text", "pilcrow", Some("Mod+Alt+0")),
    c("format.h1", "Heading 1", "heading", Some("Mod+Alt+1")),
    c("format.h2", "Heading 2", "heading", Some("Mod+Alt+2")),
    c("format.h3", "Heading 3", "heading", Some("Mod+Alt+3")),
    c("format.h4", "Heading 4", "heading", Some("Mod+Alt+4")),
    c("format.h5", "Heading 5", "heading", None),
    c("format.h6", "Heading 6", "heading", None),
    c("format.bullets", "Bullets", "list-bullet", Some("Mod+Shift+8")),
    c("format.numbering", "Numbering", "list-number", Some("Mod+Shift+7")),
    c("format.checklist", "Checklist", "list-check", Some("Mod+Shift+9")),
    c("para.alignLeft", "Align left", "align-left", None),
    c("para.alignCenter", "Center", "align-center", None),
    c("para.alignRight", "Align right", "align-right", None),
    c("para.justify", "Justify", "align-justify", None),
    c("para.lineSpacing", "Line and paragraph spacing", "line-spacing", None),
    c("para.direction", "Text direction", "text-direction", None),
    c("para.ltr", "Left-to-right", "text-direction", None),
    c("para.rtl", "Right-to-left", "text-rtl", None),
    c("para.shading", "Shading", "shading", None),
    c("para.spaceBefore", "Space before paragraph", "line-spacing", None),
    c("para.spaceAfter", "Space after paragraph", "line-spacing", None),
    c("para.borderColor", "Paragraph border color", "palette", None),
    c("para.settings", "Paragraph settings", "pilcrow", None),
    c("para.advanced", "Paragraph – advanced settings", "pilcrow", None),
    c("format.quote", "Quote", "quote", Some("Mod+Shift+.")),
    c("format.indent", "Increase indent", "indent", Some("Mod+]")),
    c("format.outdent", "Decrease indent", "outdent", Some("Mod+[")),
    c("insert.table", "Table", "table", None),
    c("insert.tableDialog", "Insert table…", "table", None),
    c("table.addRow", "Add row", "plus", None),
    c("table.addCol", "Add column", "plus", None),
    c("table.delRow", "Delete row", "minus", None),
    c("table.format", "Tidy table", "table", None),
    c("insert.csv", "Table from CSV file…", "csv", None),
    c("insert.picture", "Picture from file…", "picture", None),
    c("tbl.settings", "Table settings", "table", None),
    c("tbl.fill", "Table background", "shading", None),
    c("tbl.borderColor", "Table border color", "shading", None),
    c("tbl.rowsCols", "Rows & columns", "table-rows", None),
    c("tbl.op", "Rows & columns", "table-rows", None),
    c("pic.settings", "Picture settings", "picture", None),
    c("pic.crop", "Crop", "crop", None),
    c("pic.replace", "Replace image", "replace", None),
    c("pic.rotateLeft", "Rotate left 90°", "rotate-left", None),
    c("pic.rotateRight", "Rotate right 90°", "rotate-right", None),
    c("pic.flipH", "Flip horizontally", "flip-h", None),
    c("pic.flipV", "Flip vertically", "flip-v", None),
    c("pic.actual", "Actual size", "zoom-100", None),
    c("pic.fit", "Fit to margin", "wrap-block", None),
    c("pic.wrap", "Wrapping style", "wrap-left", None),
    c("insert.pictureClipboard", "Picture from clipboard", "clipboard-rich", None),
    c("insert.pictureUrl", "Picture from URL…", "web", None),
    c("insert.video", "Video or audio…", "video", None),
    c("insert.online", "Online video…", "video", None),
    c("insert.link", "Link…", "link", Some("Mod+K")),
    c("insert.footnote", "Footnote", "footnote", Some("Mod+Alt+F")),
    c("insert.toc", "Table of contents", "toc", None),
    c("insert.pagebreak", "Page break", "page-break", Some("Mod+Enter")),
    c("insert.cover", "Cover page", "cover", None),
    c("insert.code", "Code block", "code-block", Some("Mod+Alt+C")),
    c("insert.mathInline", "Inline equation", "sigma", Some("Mod+Shift+M")),
    c("insert.math", "Equation", "math", None),
    c("insert.diagram", "Diagram", "diagram", None),
    c("insert.chart", "Chart", "chart", None),
    c("insert.callout", "Callout", "callout", None),
    c("insert.details", "Collapsible section", "chevron-down", None),
    c("insert.hr", "Horizontal line", "hr", None),
    c("insert.date", "Date & time…", "calendar", None),
    c("insert.emoji", "Emoji", "emoji", None),
    c("insert.symbol", "Symbol", "omega", None),
    c("insert.molecule", "3D molecule", "molecule", None),
    c("insert.smiles", "Chemical structure", "molecule", None),
    c("insert.import", "Include file as icon with link…", "file-text", None),
    c("insert.importPreview", "Include file as preview with link…", "file-md", None),
    c("insert.properties", "Properties (front matter)…", "front-matter", None),
    c("design.theme", "Theme", "theme", None),
    c("design.mainfont", "Document font", "font", None),
    c("design.monofont", "Document code font", "code", None),
    c("design.fontsize", "Document font size", "font-size", None),
    c("design.sansfont", "Headings font", "font", None),
    c("design.mathfont", "Formula font", "font", None),
    c("design.paper", "Paper size", "page-size", None),
    c("design.orientation", "Orientation", "orientation", None),
    c("design.margins", "Margins", "margins", None),
    c("design.watermark", "Watermark…", "watermark", None),
    c("design.numbered", "Numbered headings", "list-number", None),
    c("design.justify", "Justify text", "pilcrow", None),
    c("slides.new", "New slide", "slide-new", Some("Mod+M")),
    c("slides.layout", "Layout", "slide-layout", None),
    c("slides.duplicate", "Duplicate slide", "copy", None),
    c("slides.delete", "Delete slide", "trash", None),
    c("slides.up", "Move slide up", "chevron-up", Some("Mod+Alt+Up")),
    c("slides.down", "Move slide down", "chevron-down", Some("Mod+Alt+Down")),
    c("slides.notes", "Speaker notes", "notes", None),
    c("slides.background", "Background color…", "background", None),
    c("slides.present", "From beginning", "present", Some("F5")),
    c("slides.presentCurrent", "From current slide", "present-current", Some("Shift+F5")),
    c("slides.fromHeadings", "Convert to slides", "slides-grid", None),
    c("review.spelling", "Check spelling as you type", "spell", Some("F7")),
    c("review.wordcount", "Word count", "wordcount", Some("Mod+Shift+G")),
    c("review.accessibility", "Check accessibility", "accessibility", None),
    c("review.outline", "Outline", "outline", None),
    c("view.write", "Write", "view-write", None),
    c("view.split", "Split", "view-split", None),
    c("view.read", "Read", "view-read", None),
    c("view.nav", "Navigation", "navigation", Some("Mod+\\")),
    c("view.comments", "Comments", "comment", None),
    c("review.comment", "New comment", "comment-add", None),
    c("help.feedback", "Help & Feedback", "megaphone", None),
    c("view.lineNumbers", "Line numbers", "line-numbers", None),
    c("view.wrap", "Word wrap", "wrap", Some("Alt+Z")),
    c("view.sync", "Scroll with the cursor", "sync", None),
    c("view.zoomIn", "Zoom in", "zoom-in", None),
    c("view.zoomOut", "Zoom out", "zoom-out", None),
    c("view.zoomReset", "100%", "zoom-100", Some("Mod+0")),
    c("view.pageWidth", "Page width", "page-width", None),
    c("view.light", "Light", "sun", None),
    c("view.dark", "Dark", "moon", None),
    c("view.system", "Use system setting", "contrast", None),
    c("view.darkPage", "Dark page", "moon", None),
    c("view.ribbon", "Collapse the ribbon", "chevron-up", Some("Mod+F1")),
    c("view.focus", "Focus", "focus", Some("F11")),
    c("view.refresh", "Refresh preview", "refresh", Some("Mod+R")),
    c("view.mathEngine", "Math: KaTeX / MathJax", "math-engine", None),
    c("view.runCode", "Run code chunks", "run", None),
    c("help.search", "Search commands", "search", Some("Alt+Q")),
    c("help.shortcuts", "Keyboard shortcuts", "keyboard", Some("Mod+/")),
    c("help.markdown", "Markdown guide", "help", None),
    // KaDee AI replaces MdCraft's AI layer
    c("ai.ask", "KaDee AI", "icons/spark.svg", None),
];

pub fn get(id: &str) -> Option<&'static Cmd> {
    COMMANDS.iter().find(|c| c.id == id)
}

/// The asset path of a command icon.
pub fn icon_path(icon: &str) -> String {
    if icon.contains('/') { icon.to_string() } else { format!("md/{icon}.svg") }
}

/// The accent part of an icon, when it has one.
pub fn accent_path(icon: &str) -> Option<String> {
    (!icon.contains('/') && crate::icons::has_accent(icon)).then(|| format!("md/{icon}.a.svg"))
}

/// `Mod+Shift+S` as GPUI writes it: `secondary-shift-s`.
pub fn gpui_key(key: &str) -> String {
    key.split('+')
        .map(|p| match p {
            "Mod" => "secondary".to_string(),
            "Alt" => "alt".to_string(),
            "Shift" => "shift".to_string(),
            "Up" => "up".to_string(),
            "Down" => "down".to_string(),
            "Enter" => "enter".to_string(),
            "Space" => "space".to_string(),
            k => k.to_lowercase(),
        })
        .collect::<Vec<_>>()
        .join("-")
}

/// `Mod+Shift+S` as the platform shows it: `⌘⇧S` or `Ctrl+Shift+S`.
pub fn key_label(key: &str) -> String {
    if cfg!(target_os = "macos") {
        key.split('+')
            .map(|p| match p {
                "Mod" => "⌘".to_string(),
                "Alt" => "⌥".to_string(),
                "Shift" => "⇧".to_string(),
                "Enter" => "↩".to_string(),
                "Up" => "↑".to_string(),
                "Down" => "↓".to_string(),
                k => k.to_string(),
            })
            .collect()
    } else {
        key.replace("Mod", "Ctrl")
    }
}

/// The tooltip of a command: its label and shortcut.
pub fn tip(c: &Cmd) -> String {
    let label = c.label.trim_end_matches('…');
    match c.key {
        Some(k) => format!("{label} ({})", key_label(k)),
        None => label.to_string(),
    }
}

/// Bind the shortcuts (inside the Markdown view). File commands are left to the host app,
/// which binds its own Save, Open and New.
pub fn bind_keys(cx: &mut App) {
    let keys: Vec<KeyBinding> = COMMANDS
        .iter()
        .filter(|c| !c.id.starts_with("file.") && c.id != "help.search")
        .filter_map(|c| c.key.map(|k| KeyBinding::new(&gpui_key(k), RunMd { id: c.id }, Some(CONTEXT))))
        .collect();
    cx.bind_keys(keys);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_icons_exist() {
        let mut ids: Vec<_> = COMMANDS.iter().map(|c| c.id).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n);
        for c in COMMANDS {
            assert!(c.icon.contains('/') || crate::icons::exists(c.icon), "{} has no icon {}", c.id, c.icon);
        }
    }

    #[test]
    fn keys_translate() {
        assert_eq!(gpui_key("Mod+Shift+S"), "secondary-shift-s");
        assert_eq!(gpui_key("Mod+Alt+Up"), "secondary-alt-up");
        assert_eq!(gpui_key("F5"), "f5");
    }
}
