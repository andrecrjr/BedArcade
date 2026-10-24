// Compiled by both egui integrations so the Windows and Wayland layouts stay identical.
use super::{egui, keyboard};
use egui::{Align2, Color32, FontId, Rect, Stroke, StrokeKind, Vec2};
use keyboard::{KEY_ROWS, KeyboardKey, KeyboardSession, Text, key_width_quarters};

#[allow(deprecated)] // show_inside bridges egui 0.34 (software) and 0.37 (eframe).
pub fn show(ui: &mut egui::Ui, session: &KeyboardSession) {
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(Color32::TRANSPARENT))
        .show_inside(ui, |ui| {
            if !session.open {
                return;
            }
            let (width, _) = session.settings.surface_size();
            let scale = (width as f32 / 720.0)
                .min(ui.available_width() / 720.0)
                .min(ui.available_height() / 258.0);
            let origin = ui.min_rect().min;
            let point = |x, y| origin + Vec2::new(x * scale, y * scale);
            let painter = ui.painter();
            let accent = Color32::from_rgb(94, 234, 212);
            let outer = Rect::from_min_size(origin, Vec2::new(720.0, 258.0) * scale);
            painter.rect_filled(
                outer,
                10.0,
                Color32::from_rgba_unmultiplied(16, 21, 31, session.settings.opacity),
            );
            painter.rect_stroke(
                outer.shrink(0.5),
                10.0,
                Stroke::new(1.0_f32, Color32::from_gray(75)),
                StrokeKind::Inside,
            );
            let title = if session.customizing {
                session.locale.text(Text::KeyboardSettingsTitle)
            } else {
                "QWERTY · F1–F12"
            };
            painter.text(
                point(12.0, 17.0),
                Align2::LEFT_CENTER,
                title,
                FontId::proportional(12.0 * scale),
                accent,
            );
            let active = if session.customizing {
                session.locale.text(Text::KeyboardMoveAdjust).to_owned()
            } else {
                format!(
                    "{}{}{}{}{}",
                    if session.modifiers.control {
                        "Ctrl + "
                    } else {
                        ""
                    },
                    if session.modifiers.alt { "Alt + " } else { "" },
                    if session.modifiers.meta {
                        "Super + "
                    } else {
                        ""
                    },
                    if session.caps_lock { "Caps · " } else { "" },
                    session
                        .selected_key()
                        .label_in(session.shifted(session.selected_key()), session.locale)
                )
            };
            painter.text(
                point(708.0, 17.0),
                Align2::RIGHT_CENTER,
                active,
                FontId::proportional(13.0 * scale),
                Color32::WHITE,
            );

            if session.customizing {
                for index in 0..6 {
                    let rect = Rect::from_min_size(
                        point(12.0, 36.0 + index as f32 * 28.0),
                        Vec2::new(696.0, 25.0) * scale,
                    );
                    let selected = session.selected_setting == index;
                    painter.rect_filled(
                        rect,
                        4.0,
                        if selected {
                            Color32::from_rgb(28, 86, 83)
                        } else {
                            Color32::from_rgb(32, 40, 54)
                        },
                    );
                    painter.text(
                        rect.left_center() + Vec2::new(10.0 * scale, 0.0),
                        Align2::LEFT_CENTER,
                        session.settings.description_in(session.locale, index),
                        FontId::proportional(13.0 * scale),
                        if selected { accent } else { Color32::WHITE },
                    );
                }
            } else {
                let unit = 696.0 / 15.0;
                for (row_index, row) in KEY_ROWS.iter().enumerate() {
                    let mut x = 12.0;
                    for (column, key) in row.iter().copied().enumerate() {
                        let units = f32::from(key_width_quarters(row_index, column)) / 4.0;
                        let rect = Rect::from_min_size(
                            point(x, 36.0 + row_index as f32 * 32.0),
                            Vec2::new(units * unit - 4.0, 28.0) * scale,
                        );
                        let selected =
                            session.selected_row == row_index && session.selected_column == column;
                        let active = match key {
                            KeyboardKey::Shift => session.shift,
                            KeyboardKey::CapsLock => session.caps_lock,
                            KeyboardKey::Control => session.modifiers.control,
                            KeyboardKey::Alt => session.modifiers.alt,
                            KeyboardKey::Meta => session.modifiers.meta,
                            _ => false,
                        };
                        let fill = if selected {
                            Color32::from_rgb(22, 112, 105)
                        } else if active {
                            Color32::from_rgb(56, 81, 75)
                        } else if matches!(key, KeyboardKey::Character(_)) {
                            Color32::from_rgb(39, 48, 63)
                        } else {
                            Color32::from_rgb(49, 59, 74)
                        };
                        painter.rect_filled(rect, 4.0, fill);
                        painter.rect_stroke(
                            rect,
                            4.0,
                            Stroke::new(
                                if selected { 2.0_f32 } else { 1.0_f32 },
                                if selected || active {
                                    accent
                                } else {
                                    Color32::from_rgb(65, 76, 94)
                                },
                            ),
                            StrokeKind::Inside,
                        );
                        let label = match key {
                            KeyboardKey::Character('a'..='z') => {
                                key.label_in(false, session.locale).to_ascii_uppercase()
                            }
                            _ => key.label_in(session.shifted(key), session.locale),
                        };
                        let font_size = if matches!(key, KeyboardKey::Character(_)) {
                            17.0
                        } else {
                            12.0
                        };
                        painter.text(
                            rect.center(),
                            Align2::CENTER_CENTER,
                            label,
                            FontId::proportional(font_size * scale),
                            Color32::WHITE,
                        );
                        if let KeyboardKey::Character(character) = key
                            && !character.is_ascii_alphabetic()
                        {
                            let alternate = key.label_in(!session.shift, session.locale);
                            painter.text(
                                rect.min + Vec2::new(5.0, 3.0) * scale,
                                Align2::LEFT_TOP,
                                alternate,
                                FontId::proportional(8.0 * scale),
                                Color32::from_gray(183),
                            );
                        }
                        x += units * unit;
                    }
                }
            }
            let help = if session.customizing {
                session.locale.text(Text::KeyboardCustomizeHelp)
            } else {
                session.locale.text(Text::KeyboardTypingHelp)
            };
            painter.text(
                point(360.0, 233.0),
                Align2::CENTER_CENTER,
                help,
                FontId::proportional(11.0 * scale),
                Color32::from_gray(210),
            );
            if !session.customizing {
                painter.text(
                    point(360.0, 249.0),
                    Align2::CENTER_CENTER,
                    session.locale.text(Text::KeyboardNavigationHelp),
                    FontId::proportional(10.0 * scale),
                    Color32::from_gray(165),
                );
            }
        });
}

#[cfg(test)]
mod sizing_tests {
    use super::*;

    #[test]
    fn all_keys_fit_when_the_monitor_requires_a_smaller_surface() {
        let context = egui::Context::default();
        let mut session = KeyboardSession::default();
        session.open = true;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(460.0, 164.0),
                )),
                ..Default::default()
            },
            |ui| show(ui, &session),
        );
        output.textures_delta.clear();
        for shape in output.shapes {
            let bounds = shape.shape.visual_bounding_rect();
            if bounds.is_positive() {
                assert!(
                    bounds.max.x <= 461.0 && bounds.max.y <= 165.0,
                    "clipped keyboard: {bounds:?}"
                );
            }
        }
    }
}
