//! Keyboard shortcuts: kd-commands' key map becomes GPUI key bindings.
//!
//! Every shortcut dispatches one [`RunCommand`] action carrying the command's stable id
//! (`file.new.markdown`); the window runs it like a click. GPUI handles sequences (`Cmd+N M`)
//! itself, with the same one-second pause kd-commands uses.

use gpui::{Action, App, KeyBinding, SharedString};
use kd_commands::{Chord, Command, Keymap, Os, display_sequence};

/// Run the command with this id.
#[derive(Clone, PartialEq, Debug, Action)]
#[action(namespace = kadee, no_json)]
pub struct RunCommand {
    pub id: SharedString,
}

gpui::actions!(
    kadee,
    [
        /// Close the menu, dialog or Settings on top.
        Dismiss,
        Quit,
        About,
    ]
);

/// The key context of the main window.
pub const CONTEXT: &str = "KadeeStudio";

/// GPUI's spelling of one chord: `cmd-n`, `ctrl-shift-tab`, `cmd-,`.
pub fn gpui_keystroke(chord: &Chord) -> String {
    let mut s = String::new();
    if chord.ctrl {
        s.push_str("ctrl-");
    }
    if chord.alt {
        s.push_str("alt-");
    }
    if chord.shift {
        s.push_str("shift-");
    }
    if chord.meta {
        // the Cmd key on the Mac, Super / Windows elsewhere: GPUI calls it the platform key
        s.push_str(if cfg!(target_os = "macos") { "cmd-" } else { "super-" });
    }
    let key = match chord.key.as_str() {
        "Comma" => ",".to_string(),
        "Period" => ".".to_string(),
        "Slash" => "/".to_string(),
        "Minus" => "-".to_string(),
        "Equals" | "Plus" => "=".to_string(),
        "Semicolon" => ";".to_string(),
        "Backslash" => "\\".to_string(),
        "OpenBracket" => "[".to_string(),
        "CloseBracket" => "]".to_string(),
        "ArrowUp" => "up".to_string(),
        "ArrowDown" => "down".to_string(),
        "ArrowLeft" => "left".to_string(),
        "ArrowRight" => "right".to_string(),
        k => k.to_ascii_lowercase(),
    };
    s.push_str(&key);
    s
}

/// The whole sequence: `cmd-n m`.
pub fn gpui_sequence(keys: &[Chord]) -> String {
    keys.iter().map(gpui_keystroke).collect::<Vec<_>>().join(" ")
}

/// Bind the key map (replacing earlier bindings of ours) plus the fixed keys.
pub fn bind(keymap: &Keymap, cx: &mut App) {
    cx.clear_key_bindings();
    kd_ui::input::bind_keys(cx);
    let mut bindings: Vec<KeyBinding> = keymap
        .bindings
        .iter()
        .filter(|b| !b.keys.is_empty())
        .map(|b| KeyBinding::new(&gpui_sequence(&b.keys), RunCommand { id: b.command.id().into() }, Some(CONTEXT)))
        .collect();
    bindings.push(KeyBinding::new("escape", Dismiss, Some(CONTEXT)));
    bindings.push(KeyBinding::new("secondary-q", Quit, None));
    cx.bind_keys(bindings);
}

/// The shortcut of a command for menus and tooltips (`Cmd+N`, `Ctrl+N M`).
pub fn shortcut_text(keymap: &Keymap, cmd: &Command) -> Option<String> {
    keymap.keys_for(cmd).filter(|k| !k.is_empty()).map(|k| display_sequence(k, Os::current()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequences_use_gpui_names() {
        let os = Os::Linux;
        let km = Keymap::defaults(os);
        let new_md = km.keys_for(&Command::NewDocument(kd_store::ModuleKind::Markdown)).unwrap();
        assert_eq!(gpui_sequence(new_md), "ctrl-n m");
        let settings = km.keys_for(&Command::OpenSettings(kd_commands::SettingsSection::General)).unwrap();
        assert_eq!(gpui_sequence(settings), "ctrl-,");
        let prev = km.keys_for(&Command::PreviousTab).unwrap();
        assert_eq!(gpui_sequence(prev), "ctrl-shift-tab");
    }

    #[test]
    fn every_default_binding_parses() {
        for b in Keymap::defaults(Os::current()).bindings {
            let text = gpui_sequence(&b.keys);
            for part in text.split(' ') {
                assert!(gpui::Keystroke::parse(part).is_ok(), "{text}");
            }
            assert!(Command::from_id(&b.command.id()).is_some(), "{}", b.command.id());
        }
    }
}
