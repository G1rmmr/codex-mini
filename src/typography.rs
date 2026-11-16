//! Register the same Korean-capable sans family before creating any widgets.
use std::borrow::Cow;

pub fn initialize() {
    let mut fonts = iced::advanced::graphics::text::font_system()
        .write()
        .expect("font system lock");
    fonts.load_font(Cow::Borrowed(include_bytes!(
        "../assets/fonts/Pretendard-Regular.otf"
    )));
    fonts.load_font(Cow::Borrowed(include_bytes!(
        "../assets/fonts/Pretendard-Bold.otf"
    )));
    fonts.raw().db_mut().set_sans_serif_family("Pretendard");
    // Preserve the user font override and select its actual family name.
    if let Some(data) = std::env::var_os("CODEX_MINI_FONT")
        .and_then(|p| std::fs::read(p).ok())
        .filter(|d| d.len() <= 32 * 1024 * 1024)
    {
        let existing: std::collections::HashSet<_> =
            fonts.raw().db().faces().map(|f| f.id).collect();
        fonts.load_font(Cow::Owned(data));
        let family = fonts
            .raw()
            .db()
            .faces()
            .find(|f| !existing.contains(&f.id))
            .and_then(|f| f.families.first().map(|f| f.0.clone()));
        if let Some(family) = family {
            fonts.raw().db_mut().set_sans_serif_family(family);
        }
    }
}

#[cfg(test)]
mod tests {
    use iced::advanced::graphics::text::cosmic_text::{self, fontdb};

    #[test]
    fn embedded_regular_and_bold_cover_latin_and_all_modern_hangul() {
        let mut db = fontdb::Database::new();
        db.load_font_data(include_bytes!("../assets/fonts/Pretendard-Regular.otf").to_vec());
        db.load_font_data(include_bytes!("../assets/fonts/Pretendard-Bold.otf").to_vec());
        db.set_sans_serif_family("Pretendard");
        let mut fonts = cosmic_text::FontSystem::new_with_locale_and_db("ko-KR".into(), db);
        for weight in [fontdb::Weight::NORMAL, fontdb::Weight::BOLD] {
            let id = fonts
                .db()
                .query(&fontdb::Query {
                    families: &[fontdb::Family::SansSerif],
                    weight,
                    ..Default::default()
                })
                .expect("bundled sans family");
            assert_eq!(fonts.db().face(id).unwrap().weight, weight);
            let font = fonts.get_font(id, weight).expect("font parses");
            let face = font.as_swash();
            let map = face.charmap();
            for codepoint in (0x20..=0x7e).chain(0xac00..=0xd7a3) {
                assert_ne!(
                    map.map(char::from_u32(codepoint).unwrap()),
                    0,
                    "missing U+{codepoint:04X}"
                );
            }
        }
    }
}
