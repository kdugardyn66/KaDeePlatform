//! # kd-ui — KaDee's GPUI design system
//!
//! Design tokens and the light and dark themes ([`theme`]), the icon set ([`icons`]), a text
//! input for one or several lines ([`input`]) and small widgets ([`widgets`]): badges, pills,
//! buttons, the logo and tooltips. Every KaDee app draws with these, so they look like one
//! suite. This is the only core crate that depends on GPUI.

pub mod icons;
pub mod input;
pub mod theme;
pub mod widgets;

pub use icons::Icon;
pub use input::{InputEvent, TextInput};
pub use theme::{Palette, Theme, mix};
pub use widgets::*;
