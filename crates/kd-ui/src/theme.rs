//! Design tokens for the light and dark themes.
//!
//! All colours come from [`Palette`]; views never hard-code a colour, so light and dark stay
//! consistent. The values are KadeeStudio 0.3's, so the start screen looks the same.

use gpui::{App, Global, Hsla, Rgba, WindowAppearance, rgb};
use kd_store::{DocGroup, ModuleKind, ThemePref};

/// Design tokens for one theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub dark: bool,
    /// Window background behind everything.
    pub canvas: Rgba,
    /// Tab strip.
    pub chrome: Rgba,
    /// Cards, inputs and the active tab.
    pub surface: Rgba,
    pub surface_hover: Rgba,
    pub border: Rgba,
    pub border_strong: Rgba,
    pub text: Rgba,
    pub text_muted: Rgba,
    pub text_faint: Rgba,
    pub accent: Rgba,
    pub accent_soft: Rgba,
    pub on_accent: Rgba,
    pub star: Rgba,
    pub danger: Rgba,
    pub success: Rgba,
}

const fn c(hex: u32) -> Rgba {
    Rgba { r: ((hex >> 16) & 0xFF) as f32 / 255.0, g: ((hex >> 8) & 0xFF) as f32 / 255.0, b: (hex & 0xFF) as f32 / 255.0, a: 1.0 }
}

impl Palette {
    pub const LIGHT: Palette = Palette {
        dark: false,
        canvas: c(0xF7F6F3),
        chrome: c(0xEEECE7),
        surface: c(0xFFFFFF),
        surface_hover: c(0xF1F0EC),
        border: c(0xE3E0D9),
        border_strong: c(0xC9C5BC),
        text: c(0x1E1D1A),
        text_muted: c(0x5F5C55),
        text_faint: c(0x8E8A82),
        accent: c(0x2F5DD6),
        accent_soft: c(0xE5ECFB),
        on_accent: c(0xFFFFFF),
        star: c(0xD99A06),
        danger: c(0xC23B2E),
        success: c(0x1F8A4C),
    };

    pub const DARK: Palette = Palette {
        dark: true,
        canvas: c(0x17181B),
        chrome: c(0x1F2024),
        surface: c(0x25272C),
        surface_hover: c(0x2E3036),
        border: c(0x34363C),
        border_strong: c(0x4A4D55),
        text: c(0xECEBE7),
        text_muted: c(0xA9A7A1),
        text_faint: c(0x7C7A75),
        accent: c(0x6E93F2),
        accent_soft: c(0x26314D),
        on_accent: c(0x101420),
        star: c(0xF2B82E),
        danger: c(0xF06E62),
        success: c(0x4CC27E),
    };

    /// The palette for a theme preference and the system's appearance.
    pub fn for_pref(pref: ThemePref, system: WindowAppearance) -> Palette {
        let dark = match pref {
            ThemePref::Light => false,
            ThemePref::Dark => true,
            ThemePref::System => matches!(system, WindowAppearance::Dark | WindowAppearance::VibrantDark),
        };
        if dark { Palette::DARK } else { Palette::LIGHT }
    }

    /// Colour that identifies a module (badges, card strips).
    pub fn module(&self, kind: ModuleKind) -> Rgba {
        let (l, d) = match kind {
            ModuleKind::Markdown => (0x2F5DD6, 0x6E93F2),
            ModuleKind::Pdf => (0xC23B2E, 0xF06E62),
            ModuleKind::Convert => (0x7A4CC9, 0xA986F0),
            ModuleKind::Dashboard => (0x0F866C, 0x3EC4A2),
            ModuleKind::Report => (0xB86B00, 0xF0A33A),
        };
        rgb(if self.dark { d } else { l })
    }

    /// The module colour mixed into the surface, for soft badge backgrounds.
    pub fn module_soft(&self, kind: ModuleKind) -> Rgba {
        self.soft(self.module(kind))
    }

    /// Colour of a start-screen document group.
    pub fn group(&self, group: DocGroup) -> Rgba {
        let (l, d) = match group {
            DocGroup::Markdown => return self.module(ModuleKind::Markdown),
            DocGroup::Pdf => return self.module(ModuleKind::Pdf),
            DocGroup::Dashboard => return self.module(ModuleKind::Dashboard),
            DocGroup::Report => return self.module(ModuleKind::Report),
            DocGroup::Office => (0x2B6C8F, 0x5FB0D8),
            DocGroup::Graphics => (0xB03A7A, 0xE87CB6),
            DocGroup::Media => (0x7A4CC9, 0xA986F0),
            DocGroup::Cad => (0x4F6B2E, 0x9BC26A),
        };
        rgb(if self.dark { d } else { l })
    }

    /// A colour mixed into the surface: badge backgrounds, soft fills.
    pub fn soft(&self, color: Rgba) -> Rgba {
        mix(self.surface, color, if self.dark { 0.22 } else { 0.12 })
    }

    /// A translucent black for shadows and scrims.
    pub fn shadow(&self) -> Hsla {
        gpui::hsla(0.0, 0.0, 0.0, if self.dark { 0.45 } else { 0.16 })
    }
}

/// Linear mix of two colours; `t = 0` gives `a`.
pub fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let f = |x: f32, y: f32| x + (y - x) * t;
    Rgba { r: f(a.r, b.r), g: f(a.g, b.g), b: f(a.b, b.b), a: f(a.a, b.a) }
}

/// The palette in use, as a GPUI global.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub pref: ThemePref,
    pub palette: Palette,
}

impl Global for Theme {}

impl Theme {
    /// Install (or replace) the theme for `pref`, following the system's appearance for
    /// [`ThemePref::System`].
    pub fn apply(pref: ThemePref, cx: &mut App) {
        let palette = Palette::for_pref(pref, cx.window_appearance());
        cx.set_global(Theme { pref, palette });
        cx.refresh_windows();
    }

    pub fn palette(cx: &App) -> Palette {
        cx.try_global::<Theme>().map(|t| t.palette).unwrap_or(Palette::LIGHT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lum(c: Rgba) -> f32 {
        0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b
    }

    #[test]
    fn mix_endpoints() {
        let a = c(0x000000);
        let b = c(0xC86432);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
    }

    #[test]
    fn text_contrasts_with_surface() {
        for p in [Palette::LIGHT, Palette::DARK] {
            assert!((lum(p.text) - lum(p.surface)).abs() > 0.6);
            assert!((lum(p.text_muted) - lum(p.surface)).abs() > 0.3);
        }
    }

    #[test]
    fn system_follows_appearance() {
        assert!(Palette::for_pref(ThemePref::System, WindowAppearance::Dark).dark);
        assert!(!Palette::for_pref(ThemePref::System, WindowAppearance::Light).dark);
        assert!(Palette::for_pref(ThemePref::Dark, WindowAppearance::Light).dark);
    }
}
