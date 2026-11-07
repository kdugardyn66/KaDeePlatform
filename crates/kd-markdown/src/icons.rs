//! MdCraft's icon set (its own drawings, MIT OR Apache-2.0), as SVG files under `icons/`.
//! Generated from MdCraft's `icons.js`: `name.svg` is the stroke part, `name.a.svg` the parts
//! MdCraft draws in the accent colour.

/// The bytes of an icon asset (`md/<name>.svg`), for the app's `AssetSource`.
pub fn load(path: &str) -> Option<&'static [u8]> {
    Some(match path {
        "md/accessibility.a.svg" => include_bytes!("../icons/accessibility.a.svg"),
        "md/accessibility.svg" => include_bytes!("../icons/accessibility.svg"),
        "md/anchor.svg" => include_bytes!("../icons/anchor.svg"),
        "md/arrow-left.svg" => include_bytes!("../icons/arrow-left.svg"),
        "md/audio.a.svg" => include_bytes!("../icons/audio.a.svg"),
        "md/audio.svg" => include_bytes!("../icons/audio.svg"),
        "md/background.a.svg" => include_bytes!("../icons/background.a.svg"),
        "md/background.svg" => include_bytes!("../icons/background.svg"),
        "md/bold.svg" => include_bytes!("../icons/bold.svg"),
        "md/calendar.a.svg" => include_bytes!("../icons/calendar.a.svg"),
        "md/calendar.svg" => include_bytes!("../icons/calendar.svg"),
        "md/callout.a.svg" => include_bytes!("../icons/callout.a.svg"),
        "md/callout.svg" => include_bytes!("../icons/callout.svg"),
        "md/caret.svg" => include_bytes!("../icons/caret.svg"),
        "md/chart-line.a.svg" => include_bytes!("../icons/chart-line.a.svg"),
        "md/chart-line.svg" => include_bytes!("../icons/chart-line.svg"),
        "md/chart-pie.a.svg" => include_bytes!("../icons/chart-pie.a.svg"),
        "md/chart-pie.svg" => include_bytes!("../icons/chart-pie.svg"),
        "md/chart.a.svg" => include_bytes!("../icons/chart.a.svg"),
        "md/chart.svg" => include_bytes!("../icons/chart.svg"),
        "md/check.svg" => include_bytes!("../icons/check.svg"),
        "md/chevron-down.svg" => include_bytes!("../icons/chevron-down.svg"),
        "md/chevron-left.svg" => include_bytes!("../icons/chevron-left.svg"),
        "md/chevron-right.svg" => include_bytes!("../icons/chevron-right.svg"),
        "md/chevron-up.svg" => include_bytes!("../icons/chevron-up.svg"),
        "md/clear.a.svg" => include_bytes!("../icons/clear.a.svg"),
        "md/clear.svg" => include_bytes!("../icons/clear.svg"),
        "md/clipboard-rich.a.svg" => include_bytes!("../icons/clipboard-rich.a.svg"),
        "md/clipboard-rich.svg" => include_bytes!("../icons/clipboard-rich.svg"),
        "md/clock.a.svg" => include_bytes!("../icons/clock.a.svg"),
        "md/clock.svg" => include_bytes!("../icons/clock.svg"),
        "md/code-block.a.svg" => include_bytes!("../icons/code-block.a.svg"),
        "md/code-block.svg" => include_bytes!("../icons/code-block.svg"),
        "md/code.a.svg" => include_bytes!("../icons/code.a.svg"),
        "md/code.svg" => include_bytes!("../icons/code.svg"),
        "md/command.a.svg" => include_bytes!("../icons/command.a.svg"),
        "md/command.svg" => include_bytes!("../icons/command.svg"),
        "md/contrast.a.svg" => include_bytes!("../icons/contrast.a.svg"),
        "md/contrast.svg" => include_bytes!("../icons/contrast.svg"),
        "md/copy.svg" => include_bytes!("../icons/copy.svg"),
        "md/cover.a.svg" => include_bytes!("../icons/cover.a.svg"),
        "md/cover.svg" => include_bytes!("../icons/cover.svg"),
        "md/csv.a.svg" => include_bytes!("../icons/csv.a.svg"),
        "md/csv.svg" => include_bytes!("../icons/csv.svg"),
        "md/cut.a.svg" => include_bytes!("../icons/cut.a.svg"),
        "md/cut.svg" => include_bytes!("../icons/cut.svg"),
        "md/diagram.a.svg" => include_bytes!("../icons/diagram.a.svg"),
        "md/diagram.svg" => include_bytes!("../icons/diagram.svg"),
        "md/emoji.a.svg" => include_bytes!("../icons/emoji.a.svg"),
        "md/emoji.svg" => include_bytes!("../icons/emoji.svg"),
        "md/export.a.svg" => include_bytes!("../icons/export.a.svg"),
        "md/export.svg" => include_bytes!("../icons/export.svg"),
        "md/file-md.a.svg" => include_bytes!("../icons/file-md.a.svg"),
        "md/file-md.svg" => include_bytes!("../icons/file-md.svg"),
        "md/file-plus.a.svg" => include_bytes!("../icons/file-plus.a.svg"),
        "md/file-plus.svg" => include_bytes!("../icons/file-plus.svg"),
        "md/file-slides.a.svg" => include_bytes!("../icons/file-slides.a.svg"),
        "md/file-slides.svg" => include_bytes!("../icons/file-slides.svg"),
        "md/file-text.a.svg" => include_bytes!("../icons/file-text.a.svg"),
        "md/file-text.svg" => include_bytes!("../icons/file-text.svg"),
        "md/file.svg" => include_bytes!("../icons/file.svg"),
        "md/find.a.svg" => include_bytes!("../icons/find.a.svg"),
        "md/find.svg" => include_bytes!("../icons/find.svg"),
        "md/focus.a.svg" => include_bytes!("../icons/focus.a.svg"),
        "md/focus.svg" => include_bytes!("../icons/focus.svg"),
        "md/folder-open.a.svg" => include_bytes!("../icons/folder-open.a.svg"),
        "md/folder-open.svg" => include_bytes!("../icons/folder-open.svg"),
        "md/folder.svg" => include_bytes!("../icons/folder.svg"),
        "md/font-color.a.svg" => include_bytes!("../icons/font-color.a.svg"),
        "md/font-color.svg" => include_bytes!("../icons/font-color.svg"),
        "md/font-grow.a.svg" => include_bytes!("../icons/font-grow.a.svg"),
        "md/font-grow.svg" => include_bytes!("../icons/font-grow.svg"),
        "md/font-shrink.a.svg" => include_bytes!("../icons/font-shrink.a.svg"),
        "md/font-shrink.svg" => include_bytes!("../icons/font-shrink.svg"),
        "md/font-size.a.svg" => include_bytes!("../icons/font-size.a.svg"),
        "md/font-size.svg" => include_bytes!("../icons/font-size.svg"),
        "md/font.a.svg" => include_bytes!("../icons/font.a.svg"),
        "md/font.svg" => include_bytes!("../icons/font.svg"),
        "md/footnote.a.svg" => include_bytes!("../icons/footnote.a.svg"),
        "md/footnote.svg" => include_bytes!("../icons/footnote.svg"),
        "md/front-matter.a.svg" => include_bytes!("../icons/front-matter.a.svg"),
        "md/front-matter.svg" => include_bytes!("../icons/front-matter.svg"),
        "md/heading.a.svg" => include_bytes!("../icons/heading.a.svg"),
        "md/heading.svg" => include_bytes!("../icons/heading.svg"),
        "md/help.a.svg" => include_bytes!("../icons/help.a.svg"),
        "md/help.svg" => include_bytes!("../icons/help.svg"),
        "md/highlight.svg" => include_bytes!("../icons/highlight.svg"),
        "md/home.a.svg" => include_bytes!("../icons/home.a.svg"),
        "md/home.svg" => include_bytes!("../icons/home.svg"),
        "md/hr.a.svg" => include_bytes!("../icons/hr.a.svg"),
        "md/hr.svg" => include_bytes!("../icons/hr.svg"),
        "md/indent.a.svg" => include_bytes!("../icons/indent.a.svg"),
        "md/indent.svg" => include_bytes!("../icons/indent.svg"),
        "md/info.a.svg" => include_bytes!("../icons/info.a.svg"),
        "md/info.svg" => include_bytes!("../icons/info.svg"),
        "md/italic.svg" => include_bytes!("../icons/italic.svg"),
        "md/keyboard.a.svg" => include_bytes!("../icons/keyboard.a.svg"),
        "md/keyboard.svg" => include_bytes!("../icons/keyboard.svg"),
        "md/line-numbers.a.svg" => include_bytes!("../icons/line-numbers.a.svg"),
        "md/line-numbers.svg" => include_bytes!("../icons/line-numbers.svg"),
        "md/link.a.svg" => include_bytes!("../icons/link.a.svg"),
        "md/link.svg" => include_bytes!("../icons/link.svg"),
        "md/list-bullet.a.svg" => include_bytes!("../icons/list-bullet.a.svg"),
        "md/list-bullet.svg" => include_bytes!("../icons/list-bullet.svg"),
        "md/list-check.a.svg" => include_bytes!("../icons/list-check.a.svg"),
        "md/list-check.svg" => include_bytes!("../icons/list-check.svg"),
        "md/list-number.a.svg" => include_bytes!("../icons/list-number.a.svg"),
        "md/list-number.svg" => include_bytes!("../icons/list-number.svg"),
        "md/logo.svg" => include_bytes!("../icons/logo.svg"),
        "md/margins.a.svg" => include_bytes!("../icons/margins.a.svg"),
        "md/margins.svg" => include_bytes!("../icons/margins.svg"),
        "md/math-engine.a.svg" => include_bytes!("../icons/math-engine.a.svg"),
        "md/math-engine.svg" => include_bytes!("../icons/math-engine.svg"),
        "md/math.a.svg" => include_bytes!("../icons/math.a.svg"),
        "md/math.svg" => include_bytes!("../icons/math.svg"),
        "md/minus.svg" => include_bytes!("../icons/minus.svg"),
        "md/molecule.a.svg" => include_bytes!("../icons/molecule.a.svg"),
        "md/molecule.svg" => include_bytes!("../icons/molecule.svg"),
        "md/moon.a.svg" => include_bytes!("../icons/moon.a.svg"),
        "md/moon.svg" => include_bytes!("../icons/moon.svg"),
        "md/more.svg" => include_bytes!("../icons/more.svg"),
        "md/nav-pane.a.svg" => include_bytes!("../icons/nav-pane.a.svg"),
        "md/nav-pane.svg" => include_bytes!("../icons/nav-pane.svg"),
        "md/notes.a.svg" => include_bytes!("../icons/notes.a.svg"),
        "md/notes.svg" => include_bytes!("../icons/notes.svg"),
        "md/omega.svg" => include_bytes!("../icons/omega.svg"),
        "md/orientation.a.svg" => include_bytes!("../icons/orientation.a.svg"),
        "md/orientation.svg" => include_bytes!("../icons/orientation.svg"),
        "md/outdent.a.svg" => include_bytes!("../icons/outdent.a.svg"),
        "md/outdent.svg" => include_bytes!("../icons/outdent.svg"),
        "md/outline.a.svg" => include_bytes!("../icons/outline.a.svg"),
        "md/outline.svg" => include_bytes!("../icons/outline.svg"),
        "md/page-break.a.svg" => include_bytes!("../icons/page-break.a.svg"),
        "md/page-break.svg" => include_bytes!("../icons/page-break.svg"),
        "md/page-size.a.svg" => include_bytes!("../icons/page-size.a.svg"),
        "md/page-size.svg" => include_bytes!("../icons/page-size.svg"),
        "md/page-width.a.svg" => include_bytes!("../icons/page-width.a.svg"),
        "md/page-width.svg" => include_bytes!("../icons/page-width.svg"),
        "md/palette.a.svg" => include_bytes!("../icons/palette.a.svg"),
        "md/palette.svg" => include_bytes!("../icons/palette.svg"),
        "md/paste.a.svg" => include_bytes!("../icons/paste.a.svg"),
        "md/paste.svg" => include_bytes!("../icons/paste.svg"),
        "md/pdf.a.svg" => include_bytes!("../icons/pdf.a.svg"),
        "md/pdf.svg" => include_bytes!("../icons/pdf.svg"),
        "md/pencil.a.svg" => include_bytes!("../icons/pencil.a.svg"),
        "md/pencil.svg" => include_bytes!("../icons/pencil.svg"),
        "md/picture.a.svg" => include_bytes!("../icons/picture.a.svg"),
        "md/picture.svg" => include_bytes!("../icons/picture.svg"),
        "md/pilcrow.svg" => include_bytes!("../icons/pilcrow.svg"),
        "md/pin.svg" => include_bytes!("../icons/pin.svg"),
        "md/plus.svg" => include_bytes!("../icons/plus.svg"),
        "md/present-current.a.svg" => include_bytes!("../icons/present-current.a.svg"),
        "md/present-current.svg" => include_bytes!("../icons/present-current.svg"),
        "md/present.a.svg" => include_bytes!("../icons/present.a.svg"),
        "md/present.svg" => include_bytes!("../icons/present.svg"),
        "md/printer.a.svg" => include_bytes!("../icons/printer.a.svg"),
        "md/printer.svg" => include_bytes!("../icons/printer.svg"),
        "md/quote.a.svg" => include_bytes!("../icons/quote.a.svg"),
        "md/quote.svg" => include_bytes!("../icons/quote.svg"),
        "md/recover.a.svg" => include_bytes!("../icons/recover.a.svg"),
        "md/recover.svg" => include_bytes!("../icons/recover.svg"),
        "md/redo.svg" => include_bytes!("../icons/redo.svg"),
        "md/refresh.a.svg" => include_bytes!("../icons/refresh.a.svg"),
        "md/refresh.svg" => include_bytes!("../icons/refresh.svg"),
        "md/rename.a.svg" => include_bytes!("../icons/rename.a.svg"),
        "md/rename.svg" => include_bytes!("../icons/rename.svg"),
        "md/replace.a.svg" => include_bytes!("../icons/replace.a.svg"),
        "md/replace.svg" => include_bytes!("../icons/replace.svg"),
        "md/run.a.svg" => include_bytes!("../icons/run.a.svg"),
        "md/run.svg" => include_bytes!("../icons/run.svg"),
        "md/save-as.a.svg" => include_bytes!("../icons/save-as.a.svg"),
        "md/save-as.svg" => include_bytes!("../icons/save-as.svg"),
        "md/save.a.svg" => include_bytes!("../icons/save.a.svg"),
        "md/save.svg" => include_bytes!("../icons/save.svg"),
        "md/search.svg" => include_bytes!("../icons/search.svg"),
        "md/select-all.a.svg" => include_bytes!("../icons/select-all.a.svg"),
        "md/select-all.svg" => include_bytes!("../icons/select-all.svg"),
        "md/settings.a.svg" => include_bytes!("../icons/settings.a.svg"),
        "md/settings.svg" => include_bytes!("../icons/settings.svg"),
        "md/sigma.svg" => include_bytes!("../icons/sigma.svg"),
        "md/slide-agenda.a.svg" => include_bytes!("../icons/slide-agenda.a.svg"),
        "md/slide-agenda.svg" => include_bytes!("../icons/slide-agenda.svg"),
        "md/slide-end.a.svg" => include_bytes!("../icons/slide-end.a.svg"),
        "md/slide-end.svg" => include_bytes!("../icons/slide-end.svg"),
        "md/slide-image.a.svg" => include_bytes!("../icons/slide-image.a.svg"),
        "md/slide-image.svg" => include_bytes!("../icons/slide-image.svg"),
        "md/slide-layout.a.svg" => include_bytes!("../icons/slide-layout.a.svg"),
        "md/slide-layout.svg" => include_bytes!("../icons/slide-layout.svg"),
        "md/slide-new.a.svg" => include_bytes!("../icons/slide-new.a.svg"),
        "md/slide-new.svg" => include_bytes!("../icons/slide-new.svg"),
        "md/slide-number.a.svg" => include_bytes!("../icons/slide-number.a.svg"),
        "md/slide-number.svg" => include_bytes!("../icons/slide-number.svg"),
        "md/slide-quote.a.svg" => include_bytes!("../icons/slide-quote.a.svg"),
        "md/slide-quote.svg" => include_bytes!("../icons/slide-quote.svg"),
        "md/slide-section.a.svg" => include_bytes!("../icons/slide-section.a.svg"),
        "md/slide-section.svg" => include_bytes!("../icons/slide-section.svg"),
        "md/slide-title.a.svg" => include_bytes!("../icons/slide-title.a.svg"),
        "md/slide-title.svg" => include_bytes!("../icons/slide-title.svg"),
        "md/slide-two.a.svg" => include_bytes!("../icons/slide-two.a.svg"),
        "md/slide-two.svg" => include_bytes!("../icons/slide-two.svg"),
        "md/slides-grid.a.svg" => include_bytes!("../icons/slides-grid.a.svg"),
        "md/slides-grid.svg" => include_bytes!("../icons/slides-grid.svg"),
        "md/spell.a.svg" => include_bytes!("../icons/spell.a.svg"),
        "md/spell.svg" => include_bytes!("../icons/spell.svg"),
        "md/star.svg" => include_bytes!("../icons/star.svg"),
        "md/strike.a.svg" => include_bytes!("../icons/strike.a.svg"),
        "md/strike.svg" => include_bytes!("../icons/strike.svg"),
        "md/subscript.a.svg" => include_bytes!("../icons/subscript.a.svg"),
        "md/subscript.svg" => include_bytes!("../icons/subscript.svg"),
        "md/sun.a.svg" => include_bytes!("../icons/sun.a.svg"),
        "md/sun.svg" => include_bytes!("../icons/sun.svg"),
        "md/superscript.a.svg" => include_bytes!("../icons/superscript.a.svg"),
        "md/superscript.svg" => include_bytes!("../icons/superscript.svg"),
        "md/symbol.a.svg" => include_bytes!("../icons/symbol.a.svg"),
        "md/symbol.svg" => include_bytes!("../icons/symbol.svg"),
        "md/sync.a.svg" => include_bytes!("../icons/sync.a.svg"),
        "md/sync.svg" => include_bytes!("../icons/sync.svg"),
        "md/table.a.svg" => include_bytes!("../icons/table.a.svg"),
        "md/table.svg" => include_bytes!("../icons/table.svg"),
        "md/template.a.svg" => include_bytes!("../icons/template.a.svg"),
        "md/template.svg" => include_bytes!("../icons/template.svg"),
        "md/theme.a.svg" => include_bytes!("../icons/theme.a.svg"),
        "md/theme.svg" => include_bytes!("../icons/theme.svg"),
        "md/toc.a.svg" => include_bytes!("../icons/toc.a.svg"),
        "md/toc.svg" => include_bytes!("../icons/toc.svg"),
        "md/trash.svg" => include_bytes!("../icons/trash.svg"),
        "md/underline.a.svg" => include_bytes!("../icons/underline.a.svg"),
        "md/underline.svg" => include_bytes!("../icons/underline.svg"),
        "md/undo.svg" => include_bytes!("../icons/undo.svg"),
        "md/video.a.svg" => include_bytes!("../icons/video.a.svg"),
        "md/video.svg" => include_bytes!("../icons/video.svg"),
        "md/view-read.a.svg" => include_bytes!("../icons/view-read.a.svg"),
        "md/view-read.svg" => include_bytes!("../icons/view-read.svg"),
        "md/view-split.a.svg" => include_bytes!("../icons/view-split.a.svg"),
        "md/view-split.svg" => include_bytes!("../icons/view-split.svg"),
        "md/view-write.a.svg" => include_bytes!("../icons/view-write.a.svg"),
        "md/view-write.svg" => include_bytes!("../icons/view-write.svg"),
        "md/watermark.a.svg" => include_bytes!("../icons/watermark.a.svg"),
        "md/watermark.svg" => include_bytes!("../icons/watermark.svg"),
        "md/web.a.svg" => include_bytes!("../icons/web.a.svg"),
        "md/web.svg" => include_bytes!("../icons/web.svg"),
        "md/window.a.svg" => include_bytes!("../icons/window.a.svg"),
        "md/window.svg" => include_bytes!("../icons/window.svg"),
        "md/wordcount.a.svg" => include_bytes!("../icons/wordcount.a.svg"),
        "md/wordcount.svg" => include_bytes!("../icons/wordcount.svg"),
        "md/wrap.a.svg" => include_bytes!("../icons/wrap.a.svg"),
        "md/wrap.svg" => include_bytes!("../icons/wrap.svg"),
        "md/x.svg" => include_bytes!("../icons/x.svg"),
        "md/zoom-100.a.svg" => include_bytes!("../icons/zoom-100.a.svg"),
        "md/zoom-100.svg" => include_bytes!("../icons/zoom-100.svg"),
        "md/zoom-in.a.svg" => include_bytes!("../icons/zoom-in.a.svg"),
        "md/zoom-in.svg" => include_bytes!("../icons/zoom-in.svg"),
        "md/zoom-out.a.svg" => include_bytes!("../icons/zoom-out.a.svg"),
        "md/zoom-out.svg" => include_bytes!("../icons/zoom-out.svg"),
        "md/comment.a.svg" => include_bytes!("../icons/comment.a.svg"),
        "md/comment.svg" => include_bytes!("../icons/comment.svg"),
        "md/comment-add.a.svg" => include_bytes!("../icons/comment-add.a.svg"),
        "md/comment-add.svg" => include_bytes!("../icons/comment-add.svg"),
        "md/megaphone.a.svg" => include_bytes!("../icons/megaphone.a.svg"),
        "md/megaphone.svg" => include_bytes!("../icons/megaphone.svg"),
        "md/navigation.svg" => include_bytes!("../icons/navigation.svg"),
        "md/align-left.svg" => include_bytes!("../icons/align-left.svg"),
        "md/align-center.svg" => include_bytes!("../icons/align-center.svg"),
        "md/align-right.svg" => include_bytes!("../icons/align-right.svg"),
        "md/align-justify.svg" => include_bytes!("../icons/align-justify.svg"),
        "md/line-spacing.a.svg" => include_bytes!("../icons/line-spacing.a.svg"),
        "md/line-spacing.svg" => include_bytes!("../icons/line-spacing.svg"),
        "md/text-direction.a.svg" => include_bytes!("../icons/text-direction.a.svg"),
        "md/text-direction.svg" => include_bytes!("../icons/text-direction.svg"),
        "md/text-rtl.a.svg" => include_bytes!("../icons/text-rtl.a.svg"),
        "md/text-rtl.svg" => include_bytes!("../icons/text-rtl.svg"),
        "md/shading.a.svg" => include_bytes!("../icons/shading.a.svg"),
        "md/shading.svg" => include_bytes!("../icons/shading.svg"),
        "md/eyedropper.svg" => include_bytes!("../icons/eyedropper.svg"),
        "md/rotate-left.svg" => include_bytes!("../icons/rotate-left.svg"),
        "md/rotate-right.svg" => include_bytes!("../icons/rotate-right.svg"),
        "md/flip-h.svg" => include_bytes!("../icons/flip-h.svg"),
        "md/flip-v.svg" => include_bytes!("../icons/flip-v.svg"),
        "md/crop.svg" => include_bytes!("../icons/crop.svg"),
        "md/wrap-inline.svg" => include_bytes!("../icons/wrap-inline.svg"),
        "md/wrap-left.svg" => include_bytes!("../icons/wrap-left.svg"),
        "md/wrap-right.svg" => include_bytes!("../icons/wrap-right.svg"),
        "md/wrap-block.svg" => include_bytes!("../icons/wrap-block.svg"),
        "md/opacity.svg" => include_bytes!("../icons/opacity.svg"),
        "md/table-rows.svg" => include_bytes!("../icons/table-rows.svg"),
        _ => return None,
    })
}

/// Whether `name` has an accent part.
pub fn has_accent(name: &str) -> bool {
    matches!(
        name,
        "accessibility"
            | "line-spacing"
            | "text-direction"
            | "text-rtl"
            | "shading"
            | "comment"
            | "comment-add"
            | "megaphone"
            | "audio"
            | "background"
            | "calendar"
            | "callout"
            | "chart-line"
            | "chart-pie"
            | "chart"
            | "clear"
            | "clipboard-rich"
            | "clock"
            | "code-block"
            | "code"
            | "command"
            | "contrast"
            | "cover"
            | "csv"
            | "cut"
            | "diagram"
            | "emoji"
            | "export"
            | "file-md"
            | "file-plus"
            | "file-slides"
            | "file-text"
            | "find"
            | "focus"
            | "folder-open"
            | "font-color"
            | "font-grow"
            | "font-shrink"
            | "font-size"
            | "font"
            | "footnote"
            | "front-matter"
            | "heading"
            | "help"
            | "home"
            | "hr"
            | "indent"
            | "info"
            | "keyboard"
            | "line-numbers"
            | "link"
            | "list-bullet"
            | "list-check"
            | "list-number"
            | "margins"
            | "math-engine"
            | "math"
            | "molecule"
            | "moon"
            | "nav-pane"
            | "notes"
            | "orientation"
            | "outdent"
            | "outline"
            | "page-break"
            | "page-size"
            | "page-width"
            | "palette"
            | "paste"
            | "pdf"
            | "pencil"
            | "picture"
            | "present-current"
            | "present"
            | "printer"
            | "quote"
            | "recover"
            | "refresh"
            | "rename"
            | "replace"
            | "run"
            | "save-as"
            | "save"
            | "select-all"
            | "settings"
            | "slide-agenda"
            | "slide-end"
            | "slide-image"
            | "slide-layout"
            | "slide-new"
            | "slide-number"
            | "slide-quote"
            | "slide-section"
            | "slide-title"
            | "slide-two"
            | "slides-grid"
            | "spell"
            | "strike"
            | "subscript"
            | "sun"
            | "superscript"
            | "symbol"
            | "sync"
            | "table"
            | "template"
            | "theme"
            | "toc"
            | "underline"
            | "video"
            | "view-read"
            | "view-split"
            | "view-write"
            | "watermark"
            | "web"
            | "window"
            | "wordcount"
            | "wrap"
            | "zoom-100"
            | "zoom-in"
            | "zoom-out"
    )
}

/// Whether `name` is an icon of the set.
pub fn exists(name: &str) -> bool {
    load(&format!("md/{name}.svg")).is_some()
}
