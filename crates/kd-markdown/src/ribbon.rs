//! The ribbon as MdCraft lays it out: tabs, groups, large / small / icon buttons, split
//! buttons with menus, and galleries. This is data; `view.rs` draws it.

use kd_formats::md::library;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    Large,
    Small,
    Icon,
}

/// The menus of split and drop-down buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Diagram,
    Chart,
    Callout,
    Code,
    Math,
    Table,
    NewSlide,
    Layout,
    Paper,
    Orientation,
    Margins,
    Watermark,
    Present,
    Date,
    Media,
    Highlight,
    FontColor,
    Chem,
    Symbol,
    Emoji,
    SlideBackground,
    /// The installed fonts, for the selection or the document.
    Fonts(FontTarget),
    /// The font sizes.
    Sizes(FontTarget),
    /// The File tab (the old menu).
    File,
    /// File › Open Recent: the last markdown files.
    Recent,
    /// File › Save As: the formats.
    SaveAs,
    /// File › Export.
    Export,
}

/// What a font or size menu changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontTarget {
    /// The selection (Home › Font): a `<span style>`.
    Selection,
    /// The document's body font (`mainfont`).
    DocMain,
    /// The document's code font (`monofont`).
    DocMono,
    /// The document's size (`fontsize`).
    DocSize,
}

impl FontTarget {
    /// The command a choice runs.
    pub fn command(self, size: bool) -> &'static str {
        match (self, size) {
            (FontTarget::Selection, false) => "format.fontFamily",
            (FontTarget::Selection, true) => "format.fontSize",
            (FontTarget::DocMain, _) => "design.mainfont",
            (FontTarget::DocMono, _) => "design.monofont",
            (FontTarget::DocSize, _) => "design.fontsize",
        }
    }
}

/// File › Save As: (extension, label, kind, icon).
pub const SAVE_AS: [(&str, &str, &str, &str); 8] = [
    ("md", "Markdown file (.md)", "Markdown", "file-md"),
    ("docx", "Word document (.docx)", "Document", "file-text"),
    ("odt", "OpenDocument text (.odt)", "Document", "file-text"),
    ("pptx", "PowerPoint slide deck (.pptx)", "Slide deck", "file-slides"),
    ("odp", "OpenDocument presentation (.odp)", "Slide deck", "file-slides"),
    ("pdf", "Portable document (.pdf)", "PDF", "pdf"),
    ("mdd", "Markdown document template (.mdd)", "Template", "file-md"),
    ("mds", "Markdown slide deck template (.mds)", "Template", "file-slides"),
];

/// Emoji offered by Insert › Emoji (inserted as the character).
pub const EMOJI: &str = "😀 😃 😄 😁 😆 😅 😂 🙂 😉 😊 😍 🤩 😎 🤔 😐 😮 😢 😭 😡 👍 👎 👏 🙌 🙏 💪 👋 👀 🎉 🎊 🎁 🔥 ⭐ ✨ 💡 📌 📎 📝 📅 📈 📉 📊 ✅ ❌ ⚠️ ❓ ❗ 💬 🚀 🎯 🏆 💰 🔒 🔑 ⏰ ☕ 🌍 ❤️ 💙 💚";

/// Slide background colours (Slides › Background), "" is the theme's.
pub const SLIDE_BACKGROUNDS: [&str; 14] = [
    "", "#ffffff", "#f8fafc", "#fef3c7", "#dcfce7", "#dbeafe", "#ede9fe", "#fce7f3", "#111827", "#0b1220", "#1e3a8a", "#065f46", "#7c2d12",
    "#581c87",
];

/// Highlighter colours (Home › Highlight ▾).
pub const HIGHLIGHTS: [(&str, &str); 8] = [
    ("#FFFF00", "Yellow"),
    ("#00FF00", "Bright green"),
    ("#00FFFF", "Turquoise"),
    ("#FF00FF", "Pink"),
    ("#FFF2CC", "Soft yellow"),
    ("#E2EFDA", "Soft green"),
    ("#DDEBF7", "Soft blue"),
    ("#FCE4D6", "Soft orange"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Btn {
    pub cmd: &'static str,
    pub label: Option<&'static str>,
    pub size: Size,
    pub menu: Option<Menu>,
    /// The whole button opens the menu.
    pub drop_only: bool,
}

impl Btn {
    pub const fn menu(mut self, m: Menu) -> Btn {
        self.menu = Some(m);
        self
    }
    pub const fn drop(mut self, m: Menu) -> Btn {
        self.menu = Some(m);
        self.drop_only = true;
        self
    }
    pub const fn label(mut self, l: &'static str) -> Btn {
        self.label = Some(l);
        self
    }
}

const fn b(cmd: &'static str, size: Size) -> Btn {
    Btn { cmd, label: None, size, menu: None, drop_only: false }
}
pub const fn large(cmd: &'static str) -> Btn {
    b(cmd, Size::Large)
}
pub const fn small(cmd: &'static str) -> Btn {
    b(cmd, Size::Small)
}
pub const fn ico(cmd: &'static str) -> Btn {
    b(cmd, Size::Icon)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gallery {
    Styles,
    Themes,
    DeckThemes,
    Layouts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowItem {
    Btn(Btn),
    FontFamily,
    FontSize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Btn(Btn),
    Col(Vec<Btn>),
    Rows(Vec<Vec<RowItem>>),
    Gallery(Gallery),
    DocFonts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub label: &'static str,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TabId {
    /// The File tab: New, Open, Open Recent, Save, Save As, Export, Print, Close.
    File,
    Home,
    Insert,
    Design,
    Slides,
    Review,
    View,
    Table,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub id: TabId,
    pub label: &'static str,
    /// Shown only while it applies (Table: the cursor is in a table).
    pub contextual: bool,
    pub groups: Vec<Group>,
}

fn g(label: &'static str, items: Vec<Item>) -> Group {
    Group { label, items }
}

fn col(b: Vec<Btn>) -> Item {
    Item::Col(b)
}

fn row(b: Vec<Btn>) -> Vec<RowItem> {
    b.into_iter().map(RowItem::Btn).collect()
}

/// MdCraft's ribbon, tab by tab (with KaDee AI in place of MdCraft's AI).
pub fn tabs() -> Vec<Tab> {
    use Item::{Btn as B, Gallery as G};
    vec![
        Tab {
            id: TabId::File,
            label: "File",
            contextual: false,
            groups: vec![
                g("Document", vec![B(large("file.new")), B(large("file.open")), B(large("file.recent").drop(Menu::Recent))]),
                g("Save", vec![B(large("file.save")), B(large("file.saveAs").label("Save As").drop(Menu::SaveAs))]),
                g("Export", vec![B(large("file.export").drop(Menu::Export))]),
                g("Print", vec![B(large("file.print").label("Print"))]),
                g("Close", vec![B(large("file.close"))]),
            ],
        },
        Tab {
            id: TabId::Home,
            label: "Home",
            contextual: false,
            groups: vec![
                g(
                    "Clipboard",
                    vec![
                        B(large("edit.paste")),
                        col(vec![small("edit.cut"), small("edit.copy"), small("format.clear").label("Clear formatting")]),
                    ],
                ),
                g(
                    "Font",
                    vec![Item::Rows(vec![
                        vec![RowItem::FontFamily, RowItem::FontSize, RowItem::Btn(ico("format.grow")), RowItem::Btn(ico("format.shrink"))],
                        row(vec![
                            ico("format.bold"),
                            ico("format.italic"),
                            ico("format.underline"),
                            ico("format.strike"),
                            ico("format.code"),
                            ico("format.sub"),
                            ico("format.sup"),
                            ico("format.highlight").menu(Menu::Highlight),
                            ico("format.color").menu(Menu::FontColor),
                            ico("format.clear"),
                        ]),
                    ])],
                ),
                g(
                    "Paragraph",
                    vec![Item::Rows(vec![
                        row(vec![
                            ico("format.bullets"),
                            ico("format.numbering"),
                            ico("format.checklist"),
                            ico("format.outdent"),
                            ico("format.indent"),
                        ]),
                        row(vec![
                            ico("format.quote"),
                            ico("insert.hr"),
                            ico("insert.callout").menu(Menu::Callout),
                            ico("insert.code").menu(Menu::Code),
                        ]),
                    ])],
                ),
                g("Styles", vec![G(Gallery::Styles)]),
                g("Editing", vec![col(vec![small("edit.find"), small("edit.replace"), small("edit.selectAll")])]),
                g("KaDee AI", vec![B(large("ai.ask"))]),
            ],
        },
        Tab {
            id: TabId::Insert,
            label: "Insert",
            contextual: false,
            groups: vec![
                g("Pages", vec![col(vec![small("insert.cover"), small("insert.pagebreak"), small("insert.toc")])]),
                g("Tables", vec![B(large("insert.table").drop(Menu::Table))]),
                g(
                    "Illustrations",
                    vec![
                        B(large("insert.picture")),
                        B(large("insert.diagram").menu(Menu::Diagram)),
                        B(large("insert.chart").menu(Menu::Chart)),
                        col(vec![
                            small("insert.video").label("Media").drop(Menu::Media),
                            small("insert.molecule").label("Chemistry").drop(Menu::Chem),
                            small("insert.import").label("Include file"),
                        ]),
                    ],
                ),
                g(
                    "Links",
                    vec![B(large("insert.link")), col(vec![small("insert.footnote"), small("insert.details").label("Collapsible")])],
                ),
                g(
                    "Blocks",
                    vec![
                        B(large("insert.callout").menu(Menu::Callout)),
                        B(large("insert.code").menu(Menu::Code)),
                        B(large("insert.math").menu(Menu::Math)),
                    ],
                ),
                g(
                    "Text",
                    vec![
                        col(vec![
                            small("insert.date").drop(Menu::Date),
                            small("insert.emoji").drop(Menu::Emoji),
                            small("insert.symbol").drop(Menu::Symbol),
                        ]),
                        B(large("insert.properties").label("Properties")),
                    ],
                ),
            ],
        },
        Tab {
            id: TabId::Design,
            label: "Design",
            contextual: false,
            groups: vec![
                g("Document themes", vec![G(Gallery::Themes)]),
                g("Document fonts", vec![Item::DocFonts]),
                g(
                    "Page setup",
                    vec![col(vec![
                        small("design.paper").drop(Menu::Paper).label("Size"),
                        small("design.orientation").drop(Menu::Orientation),
                        small("design.margins").drop(Menu::Margins),
                    ])],
                ),
                g("Page background", vec![B(large("design.watermark").drop(Menu::Watermark))]),
                g("Formatting", vec![col(vec![small("design.numbered"), small("design.justify")])]),
            ],
        },
        Tab {
            id: TabId::Slides,
            label: "Slides",
            contextual: false,
            groups: vec![
                g(
                    "Slides",
                    vec![
                        B(large("slides.new").menu(Menu::NewSlide)),
                        G(Gallery::Layouts),
                        col(vec![small("slides.duplicate"), small("slides.delete"), small("slides.notes")]),
                        col(vec![
                            small("slides.up"),
                            small("slides.down"),
                            small("slides.background").label("Background").drop(Menu::SlideBackground),
                        ]),
                    ],
                ),
                g("Slide themes", vec![G(Gallery::DeckThemes)]),
                g("Start slide show", vec![B(large("slides.present")), B(large("slides.presentCurrent"))]),
                g("Outline", vec![B(large("slides.fromHeadings"))]),
            ],
        },
        Tab {
            id: TabId::Review,
            label: "Review",
            contextual: false,
            groups: vec![
                g("Proofing", vec![B(large("review.spelling").label("Spelling")), B(large("review.wordcount"))]),
                g("Accessibility", vec![B(large("review.accessibility"))]),
                g("Structure", vec![B(large("review.outline")), col(vec![small("table.format"), small("edit.goto")])]),
            ],
        },
        Tab {
            id: TabId::View,
            label: "View",
            contextual: false,
            groups: vec![
                g(
                    "Views",
                    vec![
                        B(large("view.write")),
                        B(large("view.split")),
                        B(large("view.read")),
                        B(large("slides.present").label("Present").menu(Menu::Present)),
                    ],
                ),
                g(
                    "Show",
                    vec![
                        col(vec![small("view.nav"), small("view.lineNumbers"), small("view.wrap")]),
                        col(vec![small("view.sync"), small("review.spelling").label("Spelling"), small("view.focus").label("Focus mode")]),
                    ],
                ),
                g(
                    "Zoom",
                    vec![B(large("view.zoomReset")), col(vec![small("view.zoomIn"), small("view.zoomOut"), small("view.pageWidth")])],
                ),
                g("Interface", vec![col(vec![small("view.light"), small("view.dark"), small("view.system")]), B(large("view.darkPage"))]),
                g("Preview", vec![col(vec![small("view.refresh"), small("view.mathEngine"), small("view.runCode")])]),
                g("Window", vec![col(vec![small("view.ribbon"), small("help.shortcuts"), small("help.search")])]),
            ],
        },
        Tab {
            id: TabId::Table,
            label: "Table",
            contextual: true,
            groups: vec![
                g("Rows & columns", vec![B(large("table.addRow")), B(large("table.addCol")), B(large("table.delRow"))]),
                g("Format", vec![B(large("table.format"))]),
            ],
        },
    ]
}

/// One entry of a menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    Item { label: String, icon: Option<&'static str>, sub: Option<String>, cmd: &'static str, arg: Option<String>, checked: bool },
    Sep,
    Header(String),
}

fn item(label: impl Into<String>, icon: &'static str, cmd: &'static str, arg: Option<String>) -> Entry {
    Entry::Item { label: label.into(), icon: Some(icon), sub: None, cmd, arg, checked: false }
}

fn checked(label: &str, icon: &'static str, cmd: &'static str, arg: &str, on: bool) -> Entry {
    Entry::Item { label: label.into(), icon: Some(icon), sub: None, cmd, arg: Some(arg.into()), checked: on }
}

/// What a menu lists; `fm` reads a front matter key of the document.
pub fn entries(menu: Menu, fm: &dyn Fn(&str) -> Option<String>) -> Vec<Entry> {
    match menu {
        Menu::Diagram => library::DIAGRAMS.iter().map(|d| item(d.name, "diagram", "insert.diagram", Some(d.id.into()))).collect(),
        Menu::Chart => library::CHARTS.iter().map(|d| item(d.name, "chart", "insert.chart", Some(d.id.into()))).collect(),
        Menu::Callout => library::CALLOUTS
            .iter()
            .map(|(id, name, text)| Entry::Item {
                label: (*name).into(),
                icon: Some("callout"),
                sub: Some((*text).into()),
                cmd: "insert.callout",
                arg: Some((*id).into()),
                checked: false,
            })
            .collect(),
        Menu::Code => {
            let mut v: Vec<Entry> =
                library::CODE_LANGUAGES.iter().map(|(id, name)| item(*name, "code-block", "insert.code", Some((*id).into()))).collect();
            v.push(Entry::Sep);
            v.push(item("Python that runs (code chunk)", "run", "insert.code", Some("python {run}".into())));
            v
        }
        Menu::Math => library::EQUATIONS
            .iter()
            .map(|(name, tex)| match tex {
                None => item(*name, "sigma", "insert.mathInline", None),
                Some(t) => Entry::Item {
                    label: (*name).into(),
                    icon: Some("math"),
                    sub: (!t.contains('\u{1}')).then(|| t.replace('\n', " ").chars().take(42).collect()),
                    cmd: "insert.math",
                    arg: Some((*t).into()),
                    checked: false,
                },
            })
            .collect(),
        Menu::Table => vec![
            Entry::Header("Insert table".into()),
            Entry::Sep,
            item("Insert table…", "table", "insert.tableDialog", None),
            item("Table from a CSV file…", "csv", "insert.csv", None),
            item("Tidy the table at the cursor", "table", "table.format", None),
        ],
        Menu::NewSlide => library::LAYOUTS.iter().map(|l| item(l.name, "slide-new", "slides.new", Some(l.id.into()))).collect(),
        Menu::Layout => library::LAYOUTS.iter().map(|l| item(l.name, "slide-layout", "slides.layout", Some(l.id.into()))).collect(),
        Menu::Paper => {
            let cur = fm("paper").unwrap_or_else(|| "a4".into());
            [("a4", "A4 (210 × 297 mm)"), ("letter", "Letter (8.5 × 11 in)"), ("a5", "A5 (148 × 210 mm)"), ("legal", "Legal (8.5 × 14 in)")]
                .iter()
                .map(|(id, l)| checked(l, "page-size", "design.paper", id, cur == *id))
                .collect()
        }
        Menu::Orientation => {
            let cur = fm("orientation").unwrap_or_else(|| "portrait".into());
            [("portrait", "Portrait"), ("landscape", "Landscape")]
                .iter()
                .map(|(id, l)| checked(l, "orientation", "design.orientation", id, cur == *id))
                .collect()
        }
        Menu::Margins => {
            let cur = fm("margins").unwrap_or_else(|| "normal".into());
            [("normal", "Normal (2.5 cm)"), ("narrow", "Narrow (1.27 cm)"), ("wide", "Wide (3.5 cm)")]
                .iter()
                .map(|(id, l)| checked(l, "margins", "design.margins", id, cur == *id))
                .collect()
        }
        Menu::Watermark => {
            let cur = fm("watermark").unwrap_or_default();
            let mut v: Vec<Entry> = [
                ("DRAFT", "Draft"),
                ("CONFIDENTIAL", "Confidential"),
                ("DO NOT COPY", "Do not copy"),
                ("SAMPLE", "Sample"),
                ("URGENT", "Urgent"),
            ]
            .iter()
            .map(|(t, l)| checked(l, "watermark", "design.watermark", t, cur == *t))
            .collect();
            v.push(Entry::Sep);
            v.push(item("Custom watermark…", "watermark", "design.watermark", None));
            v.push(item("Remove watermark", "x", "design.watermark", Some(String::new())));
            v
        }
        Menu::Present => vec![
            item("From beginning", "present", "slides.present", None),
            item("From current slide", "present-current", "slides.presentCurrent", None),
        ],
        Menu::Date => {
            let now = chrono::Local::now();
            [
                now.format("%-d %B %Y").to_string(),
                now.format("%A %-d %B %Y").to_string(),
                now.format("%Y-%m-%d").to_string(),
                now.format("%-d %b %Y, %H:%M").to_string(),
            ]
            .into_iter()
            .map(|f| Entry::Item { label: f.clone(), icon: None, sub: None, cmd: "insert.date", arg: Some(f), checked: false })
            .collect()
        }
        Menu::Media => vec![
            item("Video or audio file…", "video", "insert.video", None),
            item("Online video (YouTube, Vimeo)…", "web", "insert.online", None),
        ],
        Menu::Chem => vec![
            item("3D molecule (Mol*)", "molecule", "insert.molecule", None),
            item("Chemical structure (SMILES)", "molecule", "insert.smiles", None),
            item("Chemical equation", "math", "insert.math", Some("\\ce{CH4 + 2O2 -> CO2 + 2H2O}".into())),
        ],
        Menu::Highlight => {
            let mut v = vec![
                Entry::Header("Highlight color".into()),
                item("Highlight (==text==)", "highlight", "format.highlight", None),
                Entry::Sep,
            ];
            v.extend(HIGHLIGHTS.iter().map(|(c, n)| item(*n, "highlight", "format.highlightColor", Some((*c).into()))));
            v.push(Entry::Sep);
            v.push(item("No color", "x", "format.highlightColor", Some(String::new())));
            v
        }
        Menu::FontColor => vec![Entry::Header("Font color".into()), item("Red", "font-color", "format.color", Some("#C00000".into()))],
        Menu::Symbol | Menu::Emoji | Menu::SlideBackground | Menu::Fonts(_) | Menu::Recent => Vec::new(),
        Menu::SaveAs => SAVE_AS
            .iter()
            .map(|(ext, label, group, icon)| Entry::Item {
                label: (*label).into(),
                icon: Some(icon),
                sub: Some((*group).into()),
                cmd: "file.saveAsFormat",
                arg: Some((*ext).into()),
                checked: false,
            })
            .collect(),
        Menu::Export => vec![item("Webpage (HTML)", "web", "file.exportHtml", None)],
        Menu::Sizes(target) => {
            let cur = match target {
                FontTarget::DocSize | FontTarget::DocMain | FontTarget::DocMono => fm("fontsize"),
                FontTarget::Selection => fm("\u{0}size"),
            }
            .unwrap_or_default();
            let cmd = target.command(true);
            let mut v = vec![Entry::Item {
                label: if target == FontTarget::Selection { "Theme size".into() } else { "The theme's size".into() },
                icon: None,
                sub: None,
                cmd,
                arg: Some(String::new()),
                checked: cur.is_empty(),
            }];
            v.extend(kd_formats::md::style::SIZES.iter().map(|s| {
                let label = format!("{s}");
                Entry::Item {
                    label: label.clone(),
                    icon: None,
                    sub: None,
                    cmd,
                    arg: Some(format!("{label}pt")),
                    checked: cur == format!("{label}pt"),
                }
            }));
            v
        }
        Menu::File => vec![
            item("New…", "file-plus", "file.new", None),
            item("Open…", "folder-open", "file.open", None),
            item("Save", "save", "file.save", None),
            item("Save As…", "save-as", "file.saveAs", None),
            item("Save as template…", "save-as", "file.saveTemplate", None),
            Entry::Sep,
            item("Print…", "printer", "file.print", None),
            item("Export as web page (HTML)…", "web", "file.exportHtml", None),
            item("Export as PDF…", "pdf", "file.exportPdf", None),
            item("Copy as formatted text", "clipboard-rich", "file.copyRich", None),
            Entry::Sep,
            item("Document properties", "info", "file.info", None),
            item("Close document", "x", "file.close", None),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ribbon_button_is_a_command() {
        for t in tabs() {
            for g in &t.groups {
                for it in &g.items {
                    let btns: Vec<Btn> = match it {
                        Item::Btn(b) => vec![*b],
                        Item::Col(v) => v.clone(),
                        Item::Rows(r) => r.iter().flatten().filter_map(|x| if let RowItem::Btn(b) = x { Some(*b) } else { None }).collect(),
                        _ => vec![],
                    };
                    for b in btns {
                        assert!(crate::commands::get(b.cmd).is_some(), "{} is not a command", b.cmd);
                    }
                }
            }
        }
    }

    #[test]
    fn menus_list_their_commands() {
        let none = |_: &str| None;
        for m in [Menu::Diagram, Menu::Chart, Menu::Callout, Menu::Code, Menu::Math, Menu::Paper, Menu::Watermark, Menu::Date, Menu::File] {
            for e in entries(m, &none) {
                if let Entry::Item { cmd, .. } = e {
                    assert!(crate::commands::get(cmd).is_some(), "{cmd}");
                }
            }
        }
    }
}
