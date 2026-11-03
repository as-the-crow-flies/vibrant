pub mod regular;

use egui::{FontData, FontDefinitions, FontFamily};

const FONT_BYTES: &[u8] = include_bytes!("Phosphor.ttf");

pub fn install(fonts: &mut FontDefinitions) {
    fonts
        .font_data
        .insert("phosphor".into(), FontData::from_static(FONT_BYTES).into());

    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.insert(1, "phosphor".into());
    }
}
