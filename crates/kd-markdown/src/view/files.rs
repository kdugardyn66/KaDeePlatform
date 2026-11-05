//! The File tab's windows: New… (the Markdown templates), Open… (a file browser showing
//! `.md` and `.markdown` files) and Save As in other formats.

use std::path::{Path, PathBuf};

use gpui::{AnyElement, ClickEvent, Context, FontWeight, IntoElement, MouseButton, Window, deferred, div, prelude::*, px};
use kd_formats::export::{Format, Input, export};
use kd_formats::md::{library, templates};
use kd_ui::{Palette, button, primary_button};

use super::{MarkdownEvent, MarkdownView, icon_colors, md_icon};

/// The extensions File › Open… shows.
pub const OPEN_EXTENSIONS: [&str; 2] = ["md", "markdown"];

pub struct Browser {
    pub dir: PathBuf,
    pub entries: Vec<(PathBuf, bool)>,
    pub selected: Option<PathBuf>,
    pub show_hidden: bool,
}

/// The folders and Markdown files of `dir`, folders first, by name.
pub fn list_dir(dir: &Path, show_hidden: bool) -> Vec<(PathBuf, bool)> {
    let mut out: Vec<(PathBuf, bool)> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| show_hidden || !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')))
                .filter_map(|p| {
                    if p.is_dir() {
                        Some((p, true))
                    } else {
                        let ok = p
                            .extension()
                            .and_then(|e| e.to_str())
                            .is_some_and(|e| OPEN_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()));
                        ok.then_some((p, false))
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by_key(|(p, dir)| (!dir, p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default()));
    out
}

fn home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

impl MarkdownView {
    pub(super) fn open_browser(&mut self, cx: &mut Context<Self>) {
        let dir = self
            .path
            .as_ref()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .or_else(|| self.documents_dir.clone())
            .or_else(home)
            .unwrap_or_else(|| PathBuf::from("/"));
        self.browse_to(dir, cx);
    }

    fn browse_to(&mut self, dir: PathBuf, cx: &mut Context<Self>) {
        let show_hidden = self.browser.as_ref().is_some_and(|b| b.show_hidden);
        let entries = list_dir(&dir, show_hidden);
        self.browser = Some(Browser { dir, entries, selected: None, show_hidden });
        cx.notify();
    }

    fn open_selected(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.browser = None;
        cx.emit(MarkdownEvent::OpenPath(path));
        cx.notify();
    }

    pub(super) fn browser_card(&mut self, pal: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let b = self.browser.as_ref()?;
        let (ic, ac) = icon_colors(pal);
        let dir = b.dir.clone();
        let selected = b.selected.clone();
        // places
        let mut places: Vec<(String, PathBuf)> = Vec::new();
        if let Some(p) = self.path.as_ref().and_then(|p| p.parent()) {
            places.push(("This document's folder".into(), p.to_path_buf()));
        }
        if let Some(d) = &self.documents_dir {
            places.push(("Documents".into(), d.clone()));
        }
        if let Some(h) = home() {
            places.push(("Home".into(), h.clone()));
            for sub in ["Desktop", "Downloads"] {
                let p = h.join(sub);
                if p.is_dir() {
                    places.push((sub.into(), p));
                }
            }
        }
        let place_rows: Vec<AnyElement> = places
            .into_iter()
            .enumerate()
            .map(|(i, (label, p))| {
                let on = p == dir;
                div()
                    .id(("place", i))
                    .px(px(10.0))
                    .py(px(5.0))
                    .rounded(px(5.0))
                    .text_size(px(13.0))
                    .cursor_pointer()
                    .when(on, |d| d.bg(pal.accent_soft).text_color(pal.accent))
                    .when(!on, |d| d.hover(|d| d.bg(pal.surface_hover)))
                    .child(label)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.browse_to(p.clone(), cx)))
                    .into_any_element()
            })
            .collect();
        // the path as steps
        let mut steps: Vec<AnyElement> = Vec::new();
        let comps: Vec<PathBuf> = dir.ancestors().map(Path::to_path_buf).collect::<Vec<_>>().into_iter().rev().collect();
        for (i, p) in comps.iter().enumerate() {
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| p.to_string_lossy().to_string());
            let target = p.clone();
            if i > 0 {
                steps.push(div().text_color(pal.text_faint).child("›").into_any_element());
            }
            steps.push(
                div()
                    .id(("step", i))
                    .px(px(4.0))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .hover(|d| d.bg(pal.surface_hover))
                    .child(name)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.browse_to(target.clone(), cx)))
                    .into_any_element(),
            );
        }
        let entries = b.entries.clone();
        let empty = entries.is_empty();
        let rows: Vec<AnyElement> = entries
            .into_iter()
            .enumerate()
            .map(|(i, (p, is_dir))| {
                let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let modified = std::fs::metadata(&p)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| chrono::DateTime::<chrono::Local>::from(t).format("%-d %b %Y %H:%M").to_string())
                    .unwrap_or_default();
                let on = selected.as_ref() == Some(&p);
                let target = p.clone();
                div()
                    .id(("entry", i))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(8.0))
                    .py(px(4.0))
                    .rounded(px(5.0))
                    .cursor_pointer()
                    .when(on, |d| d.bg(pal.accent_soft))
                    .when(!on, |d| d.hover(|d| d.bg(pal.surface_hover)))
                    .child(md_icon(if is_dir { "folder" } else { "file-md" }, 18.0, ic, ac))
                    .child(div().flex_1().min_w_0().text_size(px(13.0)).child(name))
                    .child(div().text_size(px(11.5)).text_color(pal.text_faint).child(if is_dir { String::new() } else { modified }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, e: &gpui::MouseDownEvent, _, cx| {
                            if is_dir {
                                if e.click_count >= 1 {
                                    this.browse_to(target.clone(), cx);
                                }
                            } else if e.click_count >= 2 {
                                this.open_selected(target.clone(), cx);
                            } else if let Some(b) = this.browser.as_mut() {
                                b.selected = Some(target.clone());
                                cx.notify();
                            }
                        }),
                    )
                    .into_any_element()
            })
            .collect();
        let parent = dir.parent().map(Path::to_path_buf);
        let can_open = selected.is_some();
        let card = div()
            .w(px(760.0))
            .h(px(500.0))
            .flex()
            .flex_col()
            .rounded(px(12.0))
            .bg(pal.surface)
            .border_1()
            .border_color(pal.border)
            .shadow_lg()
            .occlude()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .px(px(16.0))
                    .py(px(12.0))
                    .border_b_1()
                    .border_color(pal.border)
                    .child(div().text_size(px(16.0)).font_weight(FontWeight::SEMIBOLD).child("Open"))
                    .child(div().text_size(px(12.0)).text_color(pal.text_muted).child("Markdown files (.md, .markdown)"))
                    .child(div().flex_1())
                    .child(button("browse-up", "Up", pal).h(px(28.0)).when(parent.is_none(), |d| d.opacity(0.4)).on_click(cx.listener(
                        move |this, _: &ClickEvent, _, cx| {
                            if let Some(p) = parent.clone() {
                                this.browse_to(p, cx);
                            }
                        },
                    ))),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(2.0))
                    .px(px(16.0))
                    .py(px(6.0))
                    .text_size(px(12.5))
                    .text_color(pal.text_muted)
                    .children(steps),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(
                        div()
                            .w(px(180.0))
                            .flex_none()
                            .p(px(8.0))
                            .border_r_1()
                            .border_color(pal.border)
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .children(place_rows),
                    )
                    .child(
                        div().id("browse-list").flex_1().min_w_0().overflow_y_scroll().p(px(6.0)).flex().flex_col().children(rows).when(
                            empty,
                            |d| {
                                d.child(
                                    div()
                                        .p(px(12.0))
                                        .text_size(px(13.0))
                                        .text_color(pal.text_muted)
                                        .child("No folders or Markdown files here."),
                                )
                            },
                        ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(16.0))
                    .py(px(10.0))
                    .border_t_1()
                    .border_color(pal.border)
                    .child(button("browse-system", "System dialog…", pal).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.browser = None;
                        cx.emit(MarkdownEvent::Host("file.open"));
                        cx.notify();
                    })))
                    .child(div().flex_1())
                    .child(button("browse-cancel", "Cancel", pal).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.browser = None;
                        cx.notify();
                    })))
                    .child(primary_button("browse-open", "Open", pal).when(!can_open, |d| d.opacity(0.45)).on_click(cx.listener(
                        |this, _: &ClickEvent, _, cx| {
                            if let Some(p) = this.browser.as_ref().and_then(|b| b.selected.clone()) {
                                this.open_selected(p, cx);
                            }
                        },
                    ))),
            );
        Some(overlay(
            card,
            pal,
            cx.listener(|this, _, _, cx| {
                this.browser = None;
                cx.notify();
            }),
        ))
    }

    /// File › New…: the templates to start from.
    pub(super) fn chooser_card(&mut self, pal: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.chooser {
            return None;
        }
        let tile = |id: (&'static str, usize), name: String, sub: String, colors: [u32; 3], deck: bool, pal: &Palette| {
            let (hover_bg, hover_border) = (pal.surface_hover, pal.border_strong);
            div()
                .id(id)
                .p(px(8.0))
                .flex()
                .flex_col()
                .gap(px(6.0))
                .rounded(px(8.0))
                .border_1()
                .border_color(pal.border)
                .cursor_pointer()
                .hover(move |s| s.bg(hover_bg).border_color(hover_border))
                .child(
                    div()
                        .h(px(52.0))
                        .rounded(px(5.0))
                        .border_1()
                        .border_color(pal.border)
                        .bg(if deck { gpui::rgb(colors[0]) } else { gpui::rgb(0xffffff) })
                        .px(px(8.0))
                        .py(px(6.0))
                        .flex()
                        .flex_col()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(12.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(gpui::rgb(if deck { colors[2] } else { colors[0] }))
                                .child(kd_ui::truncate(&name, 22)),
                        )
                        .child(
                            div().flex().gap(px(3.0)).children(colors.map(|c| div().flex_1().h(px(4.0)).rounded(px(1.0)).bg(gpui::rgb(c)))),
                        ),
                )
                .child(div().text_size(px(12.5)).font_weight(FontWeight::MEDIUM).child(name))
                .child(div().text_size(px(11.0)).text_color(pal.text_muted).line_height(px(14.0)).child(sub))
        };
        let mut docs: Vec<AnyElement> = Vec::new();
        let mut decks: Vec<AnyElement> = Vec::new();
        for (i, t) in templates::BUILT_IN.iter().enumerate() {
            let colors = library::theme(t.theme).map(|th| th.colors).unwrap_or([0x1f2328, 0x2f5dd6, 0xf2f2f2]);
            let deck = t.kind == templates::Kind::Presentation;
            let (name, source) = (t.name.to_string(), t.source.to_string());
            let el = tile(("tpl-new", i), t.name.to_string(), t.description.to_string(), colors, deck, pal)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.chooser = false;
                    cx.emit(MarkdownEvent::NewFromTemplate { name: name.clone(), source: source.clone() });
                    cx.notify();
                }))
                .into_any_element();
            if deck { decks.push(el) } else { docs.push(el) }
        }
        let mine: Vec<AnyElement> = templates::user_templates(&self.template_dirs)
            .into_iter()
            .enumerate()
            .map(|(i, t)| {
                let deck = t.kind == templates::Kind::Presentation;
                let path = t.path.clone();
                let name = t.name.clone();
                tile(
                    ("tpl-mine", i),
                    t.name.clone(),
                    format!("{} (.{})", t.kind.label(), t.kind.extension()),
                    [0x2f5dd6, 0x19a979, 0xe8743b],
                    deck,
                    pal,
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.chooser = false;
                    match std::fs::read_to_string(&path) {
                        Ok(source) => cx.emit(MarkdownEvent::NewFromTemplate { name: name.clone(), source }),
                        Err(e) => cx.emit(MarkdownEvent::Toast(format!("Could not read the template: {e}"))),
                    }
                    cx.notify();
                }))
                .into_any_element()
            })
            .collect();
        let section = |title: &str| {
            div()
                .mt(px(14.0))
                .mb(px(8.0))
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(pal.text_muted)
                .child(title.to_string())
        };
        let card = div()
            .w(px(860.0))
            .max_h(px(620.0))
            .flex()
            .flex_col()
            .rounded(px(12.0))
            .bg(pal.surface)
            .border_1()
            .border_color(pal.border)
            .shadow_lg()
            .occlude()
            .child(
                div()
                    .flex()
                    .items_center()
                    .px(px(18.0))
                    .py(px(12.0))
                    .border_b_1()
                    .border_color(pal.border)
                    .child(div().text_size(px(16.0)).font_weight(FontWeight::SEMIBOLD).child("New"))
                    .child(div().ml(px(10.0)).text_size(px(12.0)).text_color(pal.text_muted).child("Start from a Markdown template"))
                    .child(div().flex_1())
                    .child(button("chooser-cancel", "Cancel", pal).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.chooser = false;
                        cx.notify();
                    }))),
            )
            .child(
                div()
                    .id("chooser-body")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(18.0))
                    .pb(px(18.0))
                    .child(section("Documents"))
                    .child(div().grid().grid_cols(5).gap(px(10.0)).children(docs))
                    .child(section("Slide decks"))
                    .child(div().grid().grid_cols(5).gap(px(10.0)).children(decks))
                    .when(!mine.is_empty(), |d| {
                        d.child(section("My templates")).child(div().grid().grid_cols(5).gap(px(10.0)).children(mine))
                    }),
            );
        Some(overlay(
            card,
            pal,
            cx.listener(|this, _, _, cx| {
                this.chooser = false;
                cx.notify();
            }),
        ))
    }

    /// File › Save As in another format: a copy next to the document (the tab stays on the
    /// Markdown file). Templates (.mdd, .mds) are the Markdown text itself.
    pub(super) fn save_copy_as(&mut self, ext: &str, window: &mut Window, cx: &mut Context<Self>) {
        let dir = self
            .path
            .as_ref()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .or_else(|| self.documents_dir.clone())
            .unwrap_or_else(std::env::temp_dir);
        let stem =
            self.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Document".into());
        let name = format!("{stem}.{ext}");
        let text = self.text(cx);
        let base_dir = self.path.as_ref().and_then(|p| p.parent().map(Path::to_path_buf));
        let ext = ext.to_string();
        let rx = cx.prompt_for_new_path(&dir, Some(&name));
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(path))) = rx.await else { return };
            let path = if path.extension().is_none() { path.with_extension(&ext) } else { path };
            let chosen = path.extension().and_then(|e| e.to_str()).unwrap_or(&ext).to_ascii_lowercase();
            let bytes = match Format::from_extension(&chosen) {
                Some(f) => {
                    let input = Input { text, base_dir, author: String::new() };
                    // the conversion runs off the interface thread
                    cx.background_executor().spawn(async move { export(f, &input) }).await
                }
                None => Ok(text.into_bytes()),
            };
            let msg = match bytes.and_then(|b| std::fs::write(&path, b).map_err(|e| e.to_string())) {
                Ok(()) => format!("Saved a copy as “{}”", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()),
                Err(e) => format!("Could not save: {e}"),
            };
            let _ = this.update(cx, |_, cx| cx.emit(MarkdownEvent::Toast(msg)));
        })
        .detach();
    }
}

fn overlay(card: gpui::Div, pal: &Palette, close: impl Fn(&gpui::MouseDownEvent, &mut Window, &mut gpui::App) + 'static) -> AnyElement {
    deferred(
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::Hsla::from(pal.text).opacity(0.18))
            .child(card.on_mouse_down_out(close)),
    )
    .with_priority(4)
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::list_dir;

    #[test]
    fn lists_folders_and_markdown_only() {
        let d = tempfile::tempdir().expect("dir");
        std::fs::create_dir(d.path().join("sub")).expect("sub");
        for f in ["b.md", "a.markdown", "c.txt", "d.mdd", ".hidden.md"] {
            std::fs::write(d.path().join(f), "x").expect("write");
        }
        let names: Vec<String> = list_dir(d.path(), false)
            .into_iter()
            .map(|(p, _)| p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())
            .collect();
        assert_eq!(names, ["sub", "a.markdown", "b.md"]);
    }
}
