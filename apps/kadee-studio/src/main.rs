// A window app on Windows: no console behind it (debug builds keep one for the logs).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
//! KadeeStudio on the KaDee Platform: the GPUI desktop app.
//!
//! v0.1.0 is the standalone ("Solo, no server") start screen: tab strip, sidebar, greeting,
//! quick-start tiles, the Crafting Apps, the documents table and KaDee AI in a pane on the
//! right. The editors arrive module by module (see `docs/ROADMAP.md`).

mod app;
mod assets;
mod keys;
mod settings_state;
mod view;

use gpui::{App, Application, Bounds, Menu, MenuItem, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px, size};
use kd_commands::{Command, Page, SettingsSection};

use crate::app::KadeeApp;
use crate::keys::RunCommand;

fn cmd(c: Command) -> RunCommand {
    RunCommand { id: c.id().into() }
}

fn set_menus(cx: &mut App) {
    cx.set_menus(vec![
        Menu {
            name: "KaDeePlatform".into(),
            items: vec![
                MenuItem::action("About KaDeePlatform", cmd(Command::OpenSettings(SettingsSection::About))),
                MenuItem::separator(),
                MenuItem::action("Settings…", cmd(Command::OpenSettings(SettingsSection::General))),
                MenuItem::separator(),
                MenuItem::action("Quit KaDeePlatform", keys::Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New…", cmd(Command::ShowPage(Page::New))),
                MenuItem::action("Open…", cmd(Command::OpenDialog)),
                MenuItem::separator(),
                MenuItem::action("Close Tab", cmd(Command::CloseActiveTab)),
                MenuItem::action("Reopen Closed Tab", cmd(Command::ReopenClosedTab)),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("KaDee AI", cmd(Command::ToggleAiPane)),
                MenuItem::action("Switch Theme", cmd(Command::CycleTheme)),
                MenuItem::separator(),
                MenuItem::action("Recent", cmd(Command::ShowPage(Page::Recent))),
                MenuItem::action("Starred", cmd(Command::ShowPage(Page::Starred))),
                MenuItem::action("Apps", cmd(Command::ShowPage(Page::CraftApps))),
            ],
        },
        Menu {
            name: "Window".into(),
            items: vec![MenuItem::action("Next Tab", cmd(Command::NextTab)), MenuItem::action("Previous Tab", cmd(Command::PreviousTab))],
        },
        Menu {
            name: "Help".into(),
            items: vec![
                MenuItem::action("Keyboard Shortcuts", cmd(Command::OpenSettings(SettingsSection::Keyboard))),
                MenuItem::action("KaDeePlatform on GitHub", cmd(Command::OpenUrl(kd_store::SOURCE_REPO.into()))),
            ],
        },
    ]);
}

fn main() {
    if std::env::var_os("RUST_LOG").is_some() {
        env_logger::init();
    }
    Application::new().with_assets(assets::Assets).run(|cx: &mut App| {
        cx.on_action(|_: &keys::Quit, cx| cx.quit());
        set_menus(cx);
        let bounds = Bounds::centered(None, size(px(1440.0), px(940.0)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions { title: Some("KaDeePlatform".into()), ..Default::default() }),
                    window_min_size: Some(size(px(880.0), px(560.0))),
                    app_id: Some("be.kadee.KaDeePlatform".into()),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| KadeeApp::new(window, cx)),
            )
            .expect("open the KaDeePlatform window");
        window
            .update(cx, |app, window, cx| {
                window.focus(&app.focus);
                // documents named on the command line ("Open with")
                for path in std::env::args().skip(1).filter(|a| !a.starts_with("--")).map(std::path::PathBuf::from) {
                    if path.is_file() {
                        app.open_path(path.canonicalize().unwrap_or(path), window, cx);
                    }
                }
                cx.activate(true);
            })
            .ok();
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
    });
}
