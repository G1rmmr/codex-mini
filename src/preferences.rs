use eframe::egui::{self, Color32};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    #[default]
    #[serde(other)]
    Dark,
}

impl Theme {
    pub fn apply(self, ctx: &egui::Context) {
        ctx.set_theme(match self {
            Self::Light => egui::Theme::Light,
            Self::Dark => egui::Theme::Dark,
        });
    }
}

pub fn configure(ctx: &egui::Context, theme: Theme) {
    for mode in [egui::Theme::Dark, egui::Theme::Light] {
        let mut style = (*ctx.style_of(mode)).clone();
        let dark = mode == egui::Theme::Dark;
        style.visuals = if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        let rgb = |d: [u8; 3], l: [u8; 3]| {
            let c = if dark { d } else { l };
            Color32::from_rgb(c[0], c[1], c[2])
        };
        let canvas = rgb([25, 26, 29], [245, 246, 248]);
        let surface = rgb([34, 35, 39], [255, 255, 255]);
        let border = rgb([61, 63, 69], [167, 172, 182]);
        let muted = rgb([190, 194, 203], [80, 87, 99]);
        let hover = rgb([49, 51, 57], [224, 229, 237]);
        style.visuals.panel_fill = canvas;
        style.visuals.window_fill = surface;
        style.visuals.extreme_bg_color = canvas;
        style.visuals.selection.bg_fill = rgb([67, 77, 94], [47, 69, 102]);
        style.visuals.selection.stroke = egui::Stroke::new(1.0, Color32::WHITE);
        let text = rgb([239, 240, 243], [28, 33, 41]);
        // A global override also overrides selected RichText in buttons.
        style.visuals.override_text_color = None;
        style.visuals.widgets.noninteractive.fg_stroke.color = text;
        style.visuals.weak_text_color = Some(muted);
        style.visuals.error_fg_color = rgb([255, 153, 153], [168, 29, 45]);
        style.visuals.disabled_alpha = 0.72;
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, border);
        style.visuals.widgets.inactive.weak_bg_fill = surface;
        style.visuals.widgets.inactive.bg_fill = surface;
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, border);
        style.visuals.widgets.hovered.weak_bg_fill = hover;
        style.visuals.widgets.hovered.bg_fill = hover;
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, muted);
        style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, muted);
        style.spacing.item_spacing = egui::vec2(5.0, 4.0);
        style.spacing.button_padding = egui::vec2(6.0, 2.0);
        style.spacing.interact_size.y = 20.0;
        for widget in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
        ] {
            widget.corner_radius = egui::CornerRadius::same(6);
            widget.fg_stroke.color = text;
        }
        for (kind, size) in [
            (egui::TextStyle::Body, 12.0),
            (egui::TextStyle::Button, 11.0),
            (egui::TextStyle::Small, 10.0),
            (egui::TextStyle::Heading, 13.0),
        ] {
            style
                .text_styles
                .insert(kind, egui::FontId::proportional(size));
        }
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, egui::FontId::monospace(13.0));
        style.animation_time = 0.0;
        style.visuals.text_cursor.blink = false;
        ctx.set_style_of(mode, style);
    }
    theme.apply(ctx);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn luminance(color: Color32) -> f32 {
        let channel = |n: u8| {
            let s = f32::from(n) / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }
    fn contrast(a: Color32, b: Color32) -> f32 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
    #[test]
    fn normal_muted_error_and_selected_text_have_readable_contrast() {
        let ctx = egui::Context::default();
        configure(&ctx, Theme::Dark);
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            let style = ctx.style_of(theme);
            let v = &style.visuals;
            for background in [v.panel_fill, v.window_fill] {
                for foreground in [v.text_color(), v.weak_text_color(), v.error_fg_color] {
                    assert!(
                        contrast(foreground, background) >= 4.5,
                        "{theme:?}: {foreground:?} / {background:?}"
                    );
                }
            }
            assert!(contrast(v.selection.stroke.color, v.selection.bg_fill) >= 4.5);
        }
    }
}
