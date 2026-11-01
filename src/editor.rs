// src/editor.rs - GUI РїР»Р°РіРёРЅР° (egui, РїРѕР»РЅРѕСЃС‚СЊСЋ РєР°СЃС‚РѕРјРЅР°СЏ РѕС‚СЂРёСЃРѕРІРєР°).
//
// РЎС‚РёР»СЊ: С‚С‘РјРЅС‹Р№ Р±СЂСѓС‚Р°Р»РёР·Рј / В«Р±РµС‚РѕРЅВ». РќРёРєР°РєРёС… СЃС‚Р°РЅРґР°СЂС‚РЅС‹С… РІРёРґР¶РµС‚РѕРІ egui:
// РїРѕР»Р·СѓРЅРєРё-В«РїР»Р°РЅРєРёВ», СЃРІРµС‚РѕРґРёРѕРґРЅС‹Рµ С‚СѓРјР±Р»РµСЂС‹, РїР°РЅРµР»Рё-РїР»РёС‚С‹, С€С‚Р°РјРїС‹ СЃРµРєС†РёР№,
// С€С‚СЂРёС…-РєРѕРґ РІ С€Р°РїРєРµ, Р·РµСЂРЅРёСЃС‚Р°СЏ С„Р°РєС‚СѓСЂР° Р±РµС‚РѕРЅР°.
// РЁСЂРёС„С‚С‹: Oswald (СѓР·РєРёР№ РёРЅРґСѓСЃС‚СЂРёР°Р»СЊРЅС‹Р№, Р·Р°РіРѕР»РѕРІРєРё) + JetBrains Mono (РјРµС‚РєРё, С†РёС„СЂС‹).

use crate::PostAmbientParams;
use nih_plug::prelude::*;
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, pos2, vec2, Align2, Color32, CursorIcon, FontData, FontDefinitions,
        FontFamily, FontId, Layout, Rect, RichText, Sense, Stroke, StrokeKind, Vec2},
    resizable_window::ResizableWindow,
};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------- РїР°Р»РёС‚СЂР° В«Р±РµС‚РѕРЅВ»
const BG: Color32 = Color32::from_rgb(15, 15, 15); // С„РѕРЅ РѕРєРЅР°
const PANEL: Color32 = Color32::from_rgb(24, 24, 25); // РїР°РЅРµР»СЊ-РїР»РёС‚Р°
const PANEL_DARK: Color32 = Color32::from_rgb(18, 18, 19); // РІС‹РєР»СЋС‡РµРЅРЅС‹Р№ С‚СѓРјР±Р»РµСЂ
const PANEL_HOVER: Color32 = Color32::from_rgb(33, 34, 35); // РЅР°РІРµРґРµРЅРёРµ
const LINE: Color32 = Color32::from_rgb(52, 56, 59); // Р»РёРЅРёРё, РґРѕСЂРѕР¶РєРё
const TEXT: Color32 = Color32::from_rgb(201, 206, 211); // РѕСЃРЅРѕРІРЅРѕР№ С‚РµРєСЃС‚
const MUTED: Color32 = Color32::from_rgb(120, 128, 136); // РїСЂРёРіР»СѓС€С‘РЅРЅС‹Р№
const ACCENT: Color32 = Color32::from_rgb(147, 168, 184); // С…РѕР»РѕРґРЅС‹Р№ СЃРµСЂРѕ-СЃРёРЅРёР№
const ACCENT_DIM: Color32 = Color32::from_rgb(74, 88, 99); // РїСЂРёРіР»СѓС€С‘РЅРЅС‹Р№ Р°РєС†РµРЅС‚

// ---------------------------------------------------------------- С€СЂРёС„С‚С‹
fn disp(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional) // Oswald
}
fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace) // JetBrains Mono
}
fn monob(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mono_bold".into())) // JetBrains Mono Bold
}

// РўРёРїС‹ С€СѓРјР° СЃ СЂСѓСЃСЃРєРёРјРё РёРјРµРЅР°РјРё
const KINDS: [(&str, crate::dsp::noise::NoiseType); 6] = [
    ("Р’Р•РўР•Р ", crate::dsp::noise::NoiseType::Wind),
    ("Р РђР”РРћ", crate::dsp::noise::NoiseType::Radio),
    ("Р”РћР–Р”Р¬", crate::dsp::noise::NoiseType::Rain),
    ("Р“РЈР›", crate::dsp::noise::NoiseType::Hum),
    ("РўР Р•РЎРљ", crate::dsp::noise::NoiseType::Crackle),
    ("РЎРњР•РЎР¬", crate::dsp::noise::NoiseType::Mixed),
];

// ---------------------------------------------------------------- РїСЂРёРјРёС‚РёРІС‹
/// Р—РµСЂРЅРёСЃС‚Р°СЏ С„Р°РєС‚СѓСЂР° Р±РµС‚РѕРЅР°: РґРµС‚РµСЂРјРёРЅРёСЂРѕРІР°РЅРЅС‹Рµ С‚РѕС‡РєРё СЃ РјР°Р»РѕР№ РїСЂРѕР·СЂР°С‡РЅРѕСЃС‚СЊСЋ.
fn concrete(painter: &egui::Painter, rect: Rect) {
    let mut s = 0x2545F491u32;
    let mut next = move || {
        s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        (s >> 8) as f32 / 16_777_216.0
    };
    for _ in 0..140 {
        let x = rect.left() + next() * rect.width();
        let y = rect.top() + next() * rect.height();
        let a = 4.0 + next() * 9.0;
        painter.rect_filled(
            Rect::from_center_size(pos2(x, y), vec2(1.6, 1.6)),
            0.0,
            Color32::from_white_alpha(a as u8),
        );
    }
}

/// РџСѓРЅРєС‚РёСЂРЅР°СЏ Р»РёРЅРёСЏ (РїСЂРѕРјС‹С€Р»РµРЅРЅС‹Р№ С‡РµСЂС‚С‘Р¶).
fn dashed_line(painter: &egui::Painter, y: f32, x0: f32, x1: f32, color: Color32) {
    let mut x = x0;
    while x < x1 {
        let w = ((x1 - x).min(4.0)).max(0.0);
        painter.rect_filled(Rect::from_min_size(pos2(x, y), vec2(w, 1.0)), 0.0, color);
        x += 7.0;
    }
}

/// РџРѕР»РѕСЃРєР°-С€С‚СЂРёС…РєРѕРґ РІ С€Р°РїРєРµ (РґРµС‚РµСЂРјРёРЅРёСЂРѕРІР°РЅРЅС‹Р№ СѓР·РѕСЂ).
fn barcode(painter: &egui::Painter, rect: Rect) {
    let mut s = 0x9E3779B1u32;
    let mut x = rect.right() - 76.0;
    let bottom = rect.bottom() - 5.0;
    while x < rect.right() - 2.0 {
        s = s.wrapping_mul(1103515245).wrapping_add(12345);
        let w = 1.0 + ((s >> 16) % 3) as f32;
        let h = 10.0 + ((s >> 20) % 9) as f32;
        painter.rect_filled(
            Rect::from_min_size(pos2(x, bottom - h), vec2(w, h)),
            0.0,
            Color32::from_rgb(58, 62, 66),
        );
        x += w + 2.0;
    }
}

/// РљРЅРѕРїРєР°-В«С‡РёРїВ»: РєРІР°РґСЂР°С‚РЅР°СЏ, Р±РµР· СЃРєСЂСѓРіР»РµРЅРёР№. `active` - РёРЅРІРµСЂСЃРЅР°СЏ Р·Р°Р»РёРІРєР°.
fn chip(ui: &mut egui::Ui, label: &str, active: bool, size: Vec2) -> egui::Response {
    let (resp, painter) = ui.allocate_painter(size, Sense::click());
    let rect = resp.rect;
    let hovered = resp.hovered();
    let bg = if active {
        ACCENT
    } else if hovered {
        PANEL_HOVER
    } else {
        PANEL_DARK
    };
    painter.rect_filled(rect, 0.0, bg);
    if !active {
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, LINE), StrokeKind::Inside);
    }
    let tc = if active {
        Color32::from_rgb(16, 16, 16)
    } else if hovered {
        TEXT
    } else {
        MUTED
    };
    painter.text(rect.center(), Align2::CENTER_CENTER, label, monob(9.5), tc);
    resp
}

/// РџРѕР»Р·СѓРЅРѕРє-В«РїР»Р°РЅРєР°В»: РїРѕРґРїРёСЃСЊ СЃР»РµРІР°, Р·РЅР°С‡РµРЅРёРµ СЃРїСЂР°РІР°, СЂРёСЃСѓРЅРѕРє-С‚СЂРµРє СЃРЅРёР·Сѓ.
/// РџРµСЂРµС‚Р°СЃРєРёРІР°РЅРёРµ - РїРѕ РІСЃРµР№ СЃС‚СЂРѕРєРµ, РґРІРѕР№РЅРѕР№ РєР»РёРє - СЃР±СЂРѕСЃ Рє СѓРјРѕР»С‡Р°РЅРёСЋ.
fn slider_row(ui: &mut egui::Ui, setter: &ParamSetter<'_>, param: &impl Param, label: &str) {
    let width = ui.available_width();
    let (resp, painter) = ui.allocate_painter(vec2(width, 27.0), Sense::click_and_drag());
    let rect = resp.rect;
    let hovered = resp.hovered();
    let norm = param.modulated_normalized_value();

    // РџРѕРґРїРёСЃСЊ Рё Р·РЅР°С‡РµРЅРёРµ
    let label_color = if hovered { TEXT } else { MUTED };
    painter.text(
        pos2(rect.left() + 2.0, rect.top() + 5.0),
        Align2::LEFT_TOP,
        label,
        mono(9.5),
        label_color,
    );
    painter.text(
        pos2(rect.right() - 2.0, rect.top() + 4.5),
        Align2::RIGHT_TOP,
        param.normalized_value_to_string(norm, true),
        monob(10.0),
        TEXT,
    );

    // РўСЂРµРє
    let track =
        Rect::from_min_size(pos2(rect.left(), rect.bottom() - 5.0), vec2(rect.width(), 3.0));
    painter.rect_filled(track, 0.0, LINE);
    painter.rect_filled(
        Rect::from_min_size(track.min, vec2(track.width() * norm, track.height())),
        0.0,
        if hovered { ACCENT } else { ACCENT_DIM },
    );
    // Р СѓС‡РєР°
    let handle = Rect::from_center_size(
        pos2(rect.left() + rect.width() * norm, track.center().y),
        vec2(3.0, 13.0),
    );
    painter.rect_filled(handle, 0.0, if hovered { ACCENT } else { TEXT });

    if hovered {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }
    if resp.dragged() {
        if let Some(p) = resp.interact_pointer_pos() {
            let n = ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            setter.begin_set_parameter(param);
            setter.set_parameter_normalized(param, n);
            setter.end_set_parameter(param);
        }
    }
    if resp.double_clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter_normalized(param, param.default_normalized_value());
        setter.end_set_parameter(param);
    }
}

/// РЎРІРµС‚РѕРґРёРѕРґРЅС‹Р№ С‚СѓРјР±Р»РµСЂ РґРѕСЂРѕР¶РєРё: РєРІР°РґСЂР°С‚РЅР°СЏ РїР»Р°С€РєР° + LED-РєРІР°РґСЂР°С‚.
fn toggle_row(
    ui: &mut egui::Ui,
    setter: &ParamSetter<'_>,
    param: &BoolParam,
    label: &str,
    led: Color32,
    width: f32,
) {
    let (resp, painter) = ui.allocate_painter(vec2(width, 24.0), Sense::click());
    let rect = resp.rect;
    let on = param.value();
    let hovered = resp.hovered();

    let bg = if on {
        PANEL_HOVER
    } else if hovered {
        Color32::from_rgb(28, 28, 29)
    } else {
        PANEL_DARK
    };
    painter.rect_filled(rect, 0.0, bg);
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0_f32, if on { ACCENT_DIM } else { LINE }),
        StrokeKind::Inside,
    );

    // LED
    let led_rect =
        Rect::from_center_size(pos2(rect.left() + 13.0, rect.center().y), vec2(8.0, 8.0));
    if on {
        painter.rect_filled(led_rect.expand(3.5), 0.0, led.linear_multiply(0.22));
        painter.rect_filled(led_rect, 0.0, led);
    } else {
        painter.rect_filled(led_rect, 0.0, LINE);
    }

    painter.text(
        pos2(rect.left() + 26.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        monob(10.0),
        if on { TEXT } else { MUTED },
    );

    if resp.clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, !on);
        setter.end_set_parameter(param);
    }
}

/// Р—Р°РіРѕР»РѕРІРѕРє СЃРµРєС†РёРё: С€С‚Р°РјРї-РЅРѕРјРµСЂ + РЅР°Р·РІР°РЅРёРµ + СЂР°СЃС‚СЏР¶РєР°.
fn section_title(ui: &mut egui::Ui, num: &str, title: &str) {
    ui.horizontal(|ui| {
        let (stamp, painter) = ui.allocate_painter(vec2(20.0, 14.0), Sense::hover());
        painter.rect_filled(stamp.rect, 0.0, ACCENT);
        painter.text(
            stamp.rect.center(),
            Align2::CENTER_CENTER,
            num,
            monob(9.0),
            Color32::from_rgb(16, 16, 16),
        );
        ui.label(RichText::new(title).font(disp(13.5)).color(TEXT));
    });
    ui.add_space(2.0);
}

/// РџР°РЅРµР»СЊ-В«РїР»РёС‚Р°В» СЃ С„Р°РєС‚СѓСЂРѕР№ Р±РµС‚РѕРЅР°.
fn panel(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, LINE))
        .inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            concrete(ui.painter(), ui.max_rect());
            add(ui);
        });
}

// ---------------------------------------------------------------- РѕРєРЅРѕ
pub(crate) fn create(params: Arc<PostAmbientParams>) -> Option<Box<dyn Editor>> {
    let egui_state = params.editor_state.clone();

    create_egui_editor(
        params.editor_state.clone(),
        (),
        |egui_ctx, _| {
            // РЁСЂРёС„С‚С‹: Oswald + JetBrains Mono (РєРёСЂРёР»Р»РёС†Р° РµСЃС‚СЊ РІ РѕР±РѕРёС…)
            let mut fonts = FontDefinitions::default();
            fonts
                .font_data
                .insert("display".into(), Arc::new(FontData::from_static(include_bytes!(
                    "../fonts/Display.ttf"
                ))));
            fonts
                .font_data
                .insert("mono".into(), Arc::new(FontData::from_static(include_bytes!(
                    "../fonts/Mono-Regular.ttf"
                ))));
            fonts.font_data.insert(
                "mono_bold".into(),
                Arc::new(FontData::from_static(include_bytes!(
                    "../fonts/Mono-Bold.ttf"
                ))),
            );
            fonts
                .families
                .get_mut(&FontFamily::Proportional)
                .unwrap()
                .insert(0, "display".into());
            fonts.families.insert(FontFamily::Monospace, vec!["mono".into()]);
            fonts.families.insert(
                FontFamily::Name("mono_bold".into()),
                vec!["mono_bold".into()],
            );
            egui_ctx.set_fonts(fonts);

            // РЎС‚РёР»СЊ В«Р±РµС‚РѕРЅВ»
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = BG;
            visuals.window_fill = BG;
            visuals.extreme_bg_color = PANEL;
            visuals.faint_bg_color = PANEL_DARK;
            visuals.selection.bg_fill = ACCENT;
            visuals.override_text_color = Some(TEXT);
            visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);
            visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, LINE);
            visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT_DIM);
            visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
            visuals.window_stroke = Stroke::new(1.0_f32, LINE);
            egui_ctx.set_visuals(visuals);
        },
        move |egui_ctx, setter, _state| {
            ResizableWindow::new("post-ambient-window")
                .min_size(Vec2::new(400.0, 480.0))
                .show(egui_ctx, egui_state.as_ref(), |ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
                    ui.set_min_width(360.0);

                    // ================= РЁРђРџРљРђ =================
                    let (head, painter) =
                        ui.allocate_painter(vec2(ui.available_width(), 58.0), Sense::hover());
                    let hr = head.rect;
                    painter.rect_filled(hr, 0.0, Color32::from_rgb(21, 21, 22));
                    concrete(&painter, hr);
                    painter.text(
                        pos2(hr.left(), hr.top() + 4.0),
                        Align2::LEFT_TOP,
                        "РџРћРЎРў-Р­РњР‘РР•РќРў",
                        disp(25.0),
                        TEXT,
                    );
                    painter.text(
                        pos2(hr.left() + 1.0, hr.top() + 37.0),
                        Align2::LEFT_TOP,
                        "Р®Р“Рћ-Р—РђРџРђР” РЎРџР± В· POST-PUNK В· 2000-Рµ",
                        mono(9.0),
                        MUTED,
                    );
                    painter.text(
                        pos2(hr.right() - 2.0, hr.top() + 6.0),
                        Align2::RIGHT_TOP,
                        env!("CARGO_PKG_VERSION"),
                        monob(11.0),
                        ACCENT,
                    );
                    barcode(&painter, hr);
                    painter.rect_filled(
                        Rect::from_min_size(pos2(hr.left(), hr.bottom() - 2.0), vec2(hr.width(), 2.0)),
                        0.0,
                        ACCENT_DIM,
                    );

                    ui.add_space(6.0);

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            // ============ 01 Р”РћР РћР–РљР ============
                            panel(ui, |ui| {
                                section_title(ui, "01", "Р”РћР РћР–РљР");
                                ui.horizontal(|ui| {
                                    let w = (ui.available_width() - 12.0) / 3.0;
                                    toggle_row(
                                        ui,
                                        setter,
                                        &params.noise_on,
                                        "РЁРЈРњ",
                                        Color32::from_rgb(170, 180, 188),
                                        w,
                                    );
                                    toggle_row(
                                        ui,
                                        setter,
                                        &params.bass_on,
                                        "Р‘РђРЎ",
                                        Color32::from_rgb(130, 158, 186),
                                        w,
                                    );
                                    toggle_row(
                                        ui,
                                        setter,
                                        &params.pad_on,
                                        "РџРђР”",
                                        Color32::from_rgb(186, 172, 148),
                                        w,
                                    );
                                });
                            });
                            ui.add_space(5.0);

                            // ============ 02 РЁРЈРњ ============
                            panel(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (stamp, painter) =
                                        ui.allocate_painter(vec2(20.0, 14.0), Sense::hover());
                                    painter.rect_filled(stamp.rect, 0.0, ACCENT);
                                    painter.text(
                                        stamp.rect.center(),
                                        Align2::CENTER_CENTER,
                                        "02",
                                        monob(9.0),
                                        Color32::from_rgb(16, 16, 16),
                                    );
                                    ui.label(
                                        RichText::new("РЁРЈРњ").font(disp(13.5)).color(TEXT),
                                    );
                                    ui.with_layout(
                                        Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if chip(ui, "вџі RND", false, vec2(54.0, 14.0))
                                                .clicked()
                                            {
                                                let nanos = SystemTime::now()
                                                    .duration_since(UNIX_EPOCH)
                                                    .map(|d| d.subsec_nanos())
                                                    .unwrap_or(7);
                                                let v = (nanos % 1_000_000) as i32;
                                                setter.begin_set_parameter(&params.seed);
                                                setter.set_parameter(&params.seed, v);
                                                setter.end_set_parameter(&params.seed);
                                            }
                                        },
                                    );
                                });
                                ui.add_space(2.0);

                                // РЎРµС‚РєР° С‚РёРїРѕРІ С€СѓРјР° 3x2
                                let current = params.noise_type.value();
                                for row in 0..2 {
                                    ui.horizontal(|ui| {
                                        let w = (ui.available_width() - 8.0) / 3.0;
                                        for col in 0..3 {
                                            let (name, kind) = KINDS[row * 3 + col];
                                            let active = kind == current;
                                            if chip(ui, name, active, vec2(w, 21.0)).clicked() {
                                                setter.begin_set_parameter(&params.noise_type);
                                                setter.set_parameter(&params.noise_type, kind);
                                                setter.end_set_parameter(&params.noise_type);
                                            }
                                        }
                                    });
                                }
                                ui.add_space(3.0);

                                ui.columns(2, |cols| {
                                    slider_row(
                                        &mut cols[0],
                                        setter,
                                        &params.noise_level,
                                        "РЈР РћР’Р•РќР¬",
                                    );
                                    slider_row(
                                        &mut cols[1],
                                        setter,
                                        &params.granularity,
                                        "Р—Р•Р РќРћ",
                                    );
                                });
                                ui.columns(2, |cols| {
                                    slider_row(
                                        &mut cols[0],
                                        setter,
                                        &params.modulation,
                                        "РњРћР”РЈР›РЇР¦РРЇ",
                                    );
                                    slider_row(&mut cols[1], setter, &params.seed, "Р—Р•Р РќРћ Р РќР”");
                                });
                            });
                            ui.add_space(5.0);

                            // ============ 03 Р‘РђРЎ ============
                            panel(ui, |ui| {
                                section_title(ui, "03", "Р‘РђРЎ");
                                ui.columns(2, |cols| {
                                    slider_row(&mut cols[0], setter, &params.bass_freq, "Р§РђРЎРўРћРўРђ");
                                    slider_row(&mut cols[1], setter, &params.bass_boost, "Р‘РЈРЎРў");
                                });
                                ui.columns(2, |cols| {
                                    slider_row(&mut cols[0], setter, &params.glide, "РЎР›РђР™Р”");
                                    slider_row(&mut cols[1], setter, &params.distortion, "Р”РРЎРўРћР РЁРќ");
                                });
                            });
                            ui.add_space(5.0);

                            // ============ 04 РђРўРњРћРЎР¤Р•Р Рђ ============
                            panel(ui, |ui| {
                                section_title(ui, "04", "РђРўРњРћРЎР¤Р•Р Рђ");
                                ui.columns(2, |cols| {
                                    slider_row(&mut cols[0], setter, &params.filter_cutoff, "РЎР Р•Р—");
                                    slider_row(
                                        &mut cols[1],
                                        setter,
                                        &params.resonance,
                                        "Р Р•Р—РћРќРђРќРЎ",
                                    );
                                });
                                ui.columns(2, |cols| {
                                    slider_row(&mut cols[0], setter, &params.echo_delay, "Р­РҐРћ");
                                    slider_row(&mut cols[1], setter, &params.reverb_size, "Р Р•Р’Р•Р Р‘");
                                });
                                slider_row(ui, setter, &params.stereo_width, "РЎРўР•Р Р•Рћ (Р”Р’РћР Р«)");
                            });
                            ui.add_space(5.0);

                            // ============ 05 РўР•РњРџ ============
                            panel(ui, |ui| {
                                section_title(ui, "05", "РўР•РњРџ");
                                ui.horizontal(|ui| {
                                    if chip(
                                        ui,
                                        if params.tempo_sync.value() {
                                            "РЎРРќРљ: Р’РљР›"
                                        } else {
                                            "РЎРРќРљ: Р’Р«РљР›"
                                        },
                                        params.tempo_sync.value(),
                                        vec2(86.0, 20.0),
                                    )
                                    .clicked()
                                    {
                                        let v = !params.tempo_sync.value();
                                        setter.begin_set_parameter(&params.tempo_sync);
                                        setter.set_parameter(&params.tempo_sync, v);
                                        setter.end_set_parameter(&params.tempo_sync);
                                    }
                                    ui.label(
                                        RichText::new("BPM РёР· С…РѕСЃС‚Р° (FL)")
                                            .font(mono(9.0))
                                            .color(MUTED),
                                    );
                                });
                                slider_row(ui, setter, &params.tempo, "РўР•РњРџ (BPM)");
                            });
                            ui.add_space(5.0);

                            // ============ 06 Р’Р«РҐРћР” ============
                            panel(ui, |ui| {
                                section_title(ui, "06", "Р’Р«РҐРћР”");
                                slider_row(ui, setter, &params.output, "Р“Р РћРњРљРћРЎРўР¬");
                            });

                            // ============ РџРћР”Р’РђР› ============
                            ui.add_space(6.0);
                            let (foot, painter) = ui.allocate_painter(
                                vec2(ui.available_width(), 18.0),
                                Sense::hover(),
                            );
                            let fr = foot.rect;
                            dashed_line(&painter, fr.top(), fr.left(), fr.right() - 120.0, LINE);
                            painter.text(
                                pos2(fr.right(), fr.center().y),
                                Align2::RIGHT_CENTER,
                                "РЁРЈРњ В· Р‘РђРЎ В· РџРђР” вЂ” РћР”РРќ Р‘Р•РўРћРќ",
                                mono(8.5),
                                MUTED,
                            );
                        });
                });
        },
    )
}
