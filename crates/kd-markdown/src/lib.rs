//! Kadee Markdown — the native rebuild of MdCraft on GPUI: MdCraft's ribbon, editor, preview,
//! navigation pane and status bar, with KaDee AI in place of MdCraft's AI layer.

pub mod commands;
pub mod editor;
pub mod icons;
pub mod media;
pub mod preview;
pub mod raster;
pub mod ribbon;
pub mod view;

pub use editor::{EditorEvent, SourceEditor};
pub use preview::{Preview, PreviewEvent};
pub use view::{MarkdownEvent, MarkdownView, ViewMode};

/// Key bindings of the editor and the commands; call at start-up.
pub fn init(cx: &mut gpui::App) {
    editor::bind_keys(cx);
    commands::bind_keys(cx);
    view::bind_present_keys(cx);
}
