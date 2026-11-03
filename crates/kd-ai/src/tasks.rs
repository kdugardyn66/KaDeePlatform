//! Ready-made AI actions per module. A module lists `tasks_for(its scope)` in its AI menu and
//! runs one with `task.request(selection_or_prompt)`; the home screen offers the "create" tasks.

use serde_json::Value;

use crate::schemas;
use crate::scope::{AppKind, Scope};
use crate::types::Request;

/// What a task's reply is.
#[derive(Debug, Clone, Copy)]
pub enum TaskOutput {
    /// Text in the scope's output format (Markdown, CSV, HTML, draw.io XML).
    Content,
    /// A short answer to show, not insert.
    Answer,
    /// JSON matching the schema.
    Json(fn() -> Value),
}

#[derive(Debug, Clone, Copy)]
pub struct Task {
    pub id: &'static str,
    pub scope: Scope,
    pub label: &'static str,
    /// Put in front of the user's input.
    pub instruction: &'static str,
    pub output: TaskOutput,
    /// Creates a new file (as opposed to working on a selection).
    pub creates: bool,
}

impl Task {
    /// The request for this task on `input` (the user's prompt, or the selected text).
    pub fn request(&self, input: &str) -> Request {
        let prompt = if self.creates {
            format!("{}\n\n{}", self.instruction, input.trim())
        } else {
            format!("{}\n\n<selection>\n{}\n</selection>", self.instruction, input.trim())
        };
        let r = Request::new(self.scope).user(prompt);
        match self.output {
            TaskOutput::Json(schema) => r.json(self.id, schema()),
            _ => r,
        }
    }
}

const fn t(id: &'static str, scope: Scope, label: &'static str, instruction: &'static str, output: TaskOutput, creates: bool) -> Task {
    Task { id, scope, label, instruction, output, creates }
}

const MD: Scope = Scope::App(AppKind::Markdown);
const PDF: Scope = Scope::App(AppKind::Pdf);
const DASH: Scope = Scope::App(AppKind::Dashboard);
const REPORT: Scope = Scope::App(AppKind::Report);
const WORD: Scope = Scope::App(AppKind::Word);
const SHEET: Scope = Scope::App(AppKind::Sheet);
const SLIDES: Scope = Scope::App(AppKind::Slides);
use TaskOutput::{Answer, Content};

pub static TASKS: &[Task] = &[
    t("word.draft", WORD, "Draft a document", "Write a complete document about the following.", Content, true),
    t(
        "word.rewrite",
        WORD,
        "Rewrite",
        "Rewrite this text to be clearer and more concise. Keep the meaning and the language.",
        Content,
        false,
    ),
    t(
        "word.proofread",
        WORD,
        "Fix spelling and grammar",
        "Correct spelling, grammar and punctuation. Change nothing else.",
        Content,
        false,
    ),
    t("word.summarize", WORD, "Summarize", "Summarize this in a short paragraph followed by up to five bullet points.", Content, false),
    t(
        "word.translate",
        WORD,
        "Translate",
        "Translate this into the language named in the request, or into English if none is named.",
        Content,
        false,
    ),
    t(
        "sheet.table",
        SHEET,
        "Create a table",
        "Create a table for the following, with realistic example rows.",
        TaskOutput::Json(schemas::table),
        true,
    ),
    t(
        "sheet.formula",
        SHEET,
        "Write a formula",
        "Write one spreadsheet formula that does the following. Reply with the formula, then one sentence explaining it.",
        Answer,
        false,
    ),
    t("sheet.explain", SHEET, "Explain formula", "Explain what this formula does, step by step, in plain language.", Answer, false),
    t(
        "sheet.analyze",
        SHEET,
        "Analyze data",
        "Describe the main patterns, outliers and totals in this data. Use short bullet points.",
        Answer,
        false,
    ),
    t(
        "slides.outline",
        SLIDES,
        "Create a presentation",
        "Create a presentation outline about the following: a title slide, then 5 to 10 content slides.",
        TaskOutput::Json(schemas::deck_outline),
        true,
    ),
    t("slides.notes", SLIDES, "Write speaker notes", "Write speaker notes (about 60 seconds of speech) for this slide.", Content, false),
    t(
        "slides.tighten",
        SLIDES,
        "Tighten slide text",
        "Shorten this slide's text to a title of at most 8 words and at most 5 bullets of at most 12 words.",
        Content,
        false,
    ),
    t(
        "pdf.summarize",
        PDF,
        "Summarize PDF",
        "Summarize the attached document: one paragraph, then the key points with page numbers.",
        Answer,
        false,
    ),
    t("pdf.ask", PDF, "Ask the PDF", "Answer the question about the attached document.", Answer, false),
    t(
        "pdf.tables",
        PDF,
        "Extract tables",
        "Extract every table in the attached document as Markdown pipe tables, each with its page number.",
        Content,
        false,
    ),
    t("md.draft", MD, "Draft a Markdown file", "Write a Markdown document about the following.", Content, true),
    t("md.tidy", MD, "Tidy Markdown", "Clean up this Markdown: consistent headings, lists and tables. Keep the content.", Content, false),
    t(
        "dash.plan",
        DASH,
        "Plan a dashboard",
        "Propose a dashboard for the following: the key measures, the chart for each, and the filters. Use a short table.",
        Answer,
        true,
    ),
    t(
        "dash.explain",
        DASH,
        "Explain these numbers",
        "Describe the main patterns, outliers and totals in this data. Use short bullet points.",
        Answer,
        false,
    ),
    t(
        "report.draft",
        REPORT,
        "Draft a report",
        "Write a complete report about the following: title, summary, numbered sections, and a table where useful.",
        Content,
        true,
    ),
    t("report.summary", REPORT, "Executive summary", "Write an executive summary of this report in at most 120 words.", Content, false),
    t("md.toc", MD, "Table of contents", "Write a linked table of contents for this Markdown.", Content, false),
];

/// The tasks a scope offers. The home screen offers every module's "create" tasks.
pub fn tasks_for(scope: Scope) -> Vec<&'static Task> {
    match scope {
        Scope::Home => TASKS.iter().filter(|t| t.creates).collect(),
        s => TASKS.iter().filter(|t| t.scope == s).collect(),
    }
}

pub fn task(id: &str) -> Option<&'static Task> {
    TASKS.iter().find(|t| t.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_module_has_tasks_and_one_create_task() {
        for k in AppKind::ALL {
            let ts = tasks_for(Scope::App(k));
            assert!(ts.len() >= 2, "{}", k.name());
            assert_eq!(ts.iter().filter(|t| t.creates).count(), if k == AppKind::Pdf { 0 } else { 1 }, "{}", k.name());
        }
        let mut ids: Vec<_> = TASKS.iter().map(|t| t.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), TASKS.len(), "task ids are unique");
    }

    #[test]
    fn requests() {
        let r = task("slides.outline").unwrap().request("Our café opening");
        assert_eq!(r.scope, Scope::App(AppKind::Slides));
        assert!(r.json.is_some());
        assert!(r.messages[0].text().ends_with("Our café opening"));
        let r = task("word.rewrite").unwrap().request("Some text");
        assert!(r.messages[0].text().contains("<selection>\nSome text\n</selection>"));
    }
}
