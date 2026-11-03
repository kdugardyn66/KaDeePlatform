//! Everything the window loads by path, compiled into the binary: kd-ui's icons and the
//! Crafting Apps' icons.

use std::borrow::Cow;

use gpui::{AssetSource, ImageSource, Resource, Result, SharedString};

pub struct Assets;

macro_rules! craft_icons {
    ($($id:literal),* $(,)?) => {
        fn craft_icon(path: &str) -> Option<&'static [u8]> {
            match path {
                $(concat!("craft/", $id, ".png") => Some(include_bytes!(concat!("../assets/craft-icons/", $id, ".png"))),)*
                _ => None,
            }
        }
        /// Ids of the Crafting Apps with an icon.
        pub const CRAFT_ICONS: &[&str] = &[$($id),*];
    };
}

craft_icons!(
    "cadcraft",
    "deckcraft",
    "designcraft",
    "effectcraft",
    "figcraft",
    "filmcraft",
    "gridcraft",
    "lightcraft",
    "mdcraft",
    "pdfcraft",
    "photocraft",
    "solidcraft",
    "soundcraft",
    "texturecraft",
    "vectorcraft",
    "wordcraft",
);

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(kd_ui::icons::load(path).or_else(|| craft_icon(path)).map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(CRAFT_ICONS.iter().map(|id| format!("craft/{id}.png")).filter(|p| p.starts_with(path)).map(SharedString::from).collect())
    }
}

/// The icon of a Crafting App, when it has one.
pub fn craft_image(id: &str) -> Option<ImageSource> {
    CRAFT_ICONS.contains(&id).then(|| ImageSource::Resource(Resource::Embedded(format!("craft/{id}.png").into())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_crafting_app_has_an_icon() {
        for app in kd_store::CRAFT_APPS.iter() {
            assert!(craft_icon(&format!("craft/{}.png", app.id)).is_some(), "{}", app.id);
        }
    }
}
