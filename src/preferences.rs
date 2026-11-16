use iced::Color;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    #[default]
    #[serde(other)]
    Dark,
}
#[derive(Clone, Copy)]
pub struct Colors {
    pub background: Color,
    pub sidebar: Color,
    pub surface: Color,
    pub hover: Color,
    pub text: Color,
    pub muted: Color,
    pub border: Color,
    pub accent: Color,
    pub on_accent: Color,
}
fn gray(value: u8) -> Color {
    Color::from_rgb8(value, value, value)
}
impl Theme {
    pub fn colors(self) -> Colors {
        match self {
            Self::Dark => Colors {
                background: gray(33),
                sidebar: gray(24),
                surface: gray(48),
                hover: gray(57),
                text: gray(240),
                muted: gray(175),
                border: gray(76),
                accent: gray(245),
                on_accent: gray(20),
            },
            Self::Light => Colors {
                background: gray(255),
                sidebar: gray(249),
                surface: gray(243),
                hover: gray(233),
                text: gray(24),
                muted: gray(100),
                border: gray(180),
                accent: gray(24),
                on_accent: gray(255),
            },
        }
    }
    pub fn iced(self) -> iced::Theme {
        let c = self.colors();
        iced::Theme::custom(
            if self == Self::Dark {
                "Codex Mini Dark"
            } else {
                "Codex Mini Light"
            },
            iced::theme::Palette {
                background: c.background,
                text: c.text,
                primary: c.accent,
                success: c.text,
                warning: c.text,
                danger: c.text,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn luminance(c: Color) -> f32 {
        let linear = |v: f32| {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
    }
    #[test]
    fn both_monochrome_themes_keep_body_and_secondary_text_readable() {
        for theme in [Theme::Dark, Theme::Light] {
            let c = theme.colors();
            for fg in [c.text, c.muted] {
                for bg in [c.background, c.sidebar, c.surface] {
                    let (a, b) = (luminance(fg), luminance(bg));
                    assert!((a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5);
                }
            }
            for color in [
                c.background,
                c.sidebar,
                c.surface,
                c.hover,
                c.text,
                c.muted,
                c.border,
                c.accent,
                c.on_accent,
            ] {
                assert!((color.r - color.g).abs() < 0.0001 && (color.g - color.b).abs() < 0.0001);
            }
        }
    }
}
