//! Markdown, in MdCraft's dialect.

pub mod comments;
pub mod diagram;
pub mod edit;
pub mod find;
pub mod highlight;
pub mod html;
pub mod library;
pub mod look;
pub mod math;
pub mod media;
pub mod page;
pub mod para;
pub mod parse;
pub mod props;
pub mod review;
pub mod slides;
pub mod style;
pub mod templates;

pub use edit::{Edit, LineKind, SEL};
pub use highlight::{LineToks, Tok, highlight};
pub use parse::{Align, Block, Heading, Inline, ListItem, Parsed, Style, inline_text, outline, parse, word_count};
