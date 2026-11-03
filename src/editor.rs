// src/editor.rs - GUI плагина (egui, полностью кастомная отрисовка).
//
// Стиль: минимализм — почти чёрный фон, тонкие серые линии, белый текст,
// один кислотный акцент (лайм). Без текстур и «бруталистских» штампов.
// Никаких стандартных виджетов egui: ползунки-«планки», квадратные
// тумблеры, панели-плиты, заголовки секций.
// Шрифты: Oswald (заголовки) + JetBrains Mono (метки, цифры).

use crate::dsp::breaks::{BreakMode, STEPS};
use crate::sample::{self, UiStatus};
use crate::PostAmbientParams;
use nih_plug::prelude::*;
use nih_plug_egui::{
    create_egui_editor,
    egui::{
        self, pos2, vec2, Align2, Color32, CursorIcon, FontData, FontDefinitions, FontFamily,
        FontId, Rect, RichText, Sense, Stroke, StrokeKind, Vec2,
    },
    resizable_window::ResizableWindow,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

// ---------------------------------------------------------------- палитра «минимализм»
const BG: Color32 = Color32::from_rgb(10, 10, 11); // почти чёрный фон
const PANEL: Color32 = Color32::from_rgb(16, 16, 18); // панель-плита
const PANEL_DARK: Color32 = Color32::from_rgb(13, 13, 15); // выключенный тумблер
const PANEL_HOVER: Color32 = Color32::from_rgb(24, 24, 27); // наведение
const LINE: Color32 = Color32::from_rgb(38, 38, 43); // тонкие серые линии
const TEXT: Color32 = Color32::from_rgb(242, 242, 243); // белый текст
const MUTED: Color32 = Color32::from_rgb(122, 122, 128); // приглушённый
const ACCENT: Color32 = Color32::from_rgb(200, 255, 0); // кислотный лайм
const ACCENT_DIM: Color32 = Color32::from_rgb(110, 138, 0); // приглушённый лайм
const LED_ON: Color32 = Color32::from_rgb(200, 255, 0); // MIDI-нота зажата
const ERR: Color32 = Color32::from_rgb(255, 92, 92); // ошибка загрузки файла

// ---------------------------------------------------------------- шрифты
fn disp(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional) // Oswald
}
fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace) // JetBrains Mono
}
fn monob(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mono_bold".into())) // JetBrains Mono Bold
}

// ---------------------------------------------------------------- примитивы
/// Кнопка-«чип»: квадратная, без скруглений. `active` - заливка лаймом.
fn chip(ui: &mut egui::Ui, label: &str, active: bool, size: Vec2) -> egui::Response {
    let (resp, painter) = ui.allocate_painter(size, Sense::click());
    let rect = resp.rect;
    let hovered = resp.hovered();
    // Активный - сплошная заливка лаймом; обычный - прозрачная с тонкой линией
    if active {
        painter.rect_filled(rect, 0.0, ACCENT);
    } else {
        if hovered {
            painter.rect_filled(rect, 0.0, PANEL_HOVER);
        }
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0_f32, if hovered { MUTED } else { LINE }),
            StrokeKind::Inside,
        );
    }
    let tc = if active {
        Color32::from_rgb(16, 16, 16)
    } else if hovered {
        TEXT
    } else {
        MUTED
    };
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        monob(9.5),
        tc,
    );
    resp
}

/// Установка параметра из пресет-сцены (в единицах самого параметра).
fn set_param<P: Param>(setter: &ParamSetter<'_>, param: &P, value: P::Plain) {
    setter.begin_set_parameter(param);
    setter.set_parameter(param, value);
    setter.end_set_parameter(param);
}

/// Ползунок-«планка»: подпись слева, значение справа, рисунок-трек снизу.
/// Перетаскивание - по всей строке, двойной клик - сброс к умолчанию.
fn slider_row(ui: &mut egui::Ui, setter: &ParamSetter<'_>, param: &impl Param, label: &str) {
    let width = ui.available_width();
    let (resp, painter) = ui.allocate_painter(vec2(width, 27.0), Sense::click_and_drag());
    let rect = resp.rect;
    let hovered = resp.hovered();
    let norm = param.modulated_normalized_value();

    // Подпись и значение
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

    // Трек (тонкая линия)
    let track =
        Rect::from_min_size(pos2(rect.left(), rect.bottom() - 5.0), vec2(rect.width(), 2.0));
    painter.rect_filled(track, 0.0, LINE);
    painter.rect_filled(
        Rect::from_min_size(track.min, vec2(track.width() * norm, track.height())),
        0.0,
        if hovered { ACCENT } else { ACCENT_DIM },
    );
    // Ручка
    let handle = Rect::from_center_size(
        pos2(rect.left() + rect.width() * norm, track.center().y),
        vec2(3.0, 11.0),
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

/// Светодиодный тумблер дорожки: квадратная плашка + LED-квадрат.
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

    let bg = if on || hovered { PANEL_HOVER } else { PANEL_DARK };
    painter.rect_filled(rect, 0.0, bg);
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0_f32, if on { ACCENT } else { LINE }),
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

/// Заголовок секции: номер-акцент + название + тонкая линия-растяжка.
fn section_title(ui: &mut egui::Ui, num: &str, title: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(num).font(monob(9.5)).color(ACCENT));
        ui.label(RichText::new(title).font(disp(13.5)).color(TEXT));
        let (l, p) = ui.allocate_painter(
            vec2(ui.available_width().max(0.0), 1.0),
            Sense::hover(),
        );
        p.rect_filled(l.rect, 0.0, LINE);
    });
    ui.add_space(2.0);
}

/// Панель-«плита»: почти чёрная заливка с тонкой серой линией.
fn panel(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::NONE
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, LINE))
        .inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            add(ui);
        });
}

/// Лента из 32 шагов брейка: текущий шаг горит лаймом,
/// яркость остальных зависит от уровня брейка.
fn step_strip(ui: &mut egui::Ui, break_step: &AtomicU32, level: f32, active: bool) {
    let (resp, painter) = ui.allocate_painter(vec2(ui.available_width(), 17.0), Sense::hover());
    let rect = resp.rect;
    let cur = break_step.load(Ordering::Relaxed) as usize;
    let gap = 2.0;
    let bw = (rect.width() - gap * (STEPS as f32 - 1.0)) / STEPS as f32;

    for i in 0..STEPS {
        let x = rect.left() + i as f32 * (bw + gap);
        let bar = Rect::from_min_size(pos2(x, rect.top()), vec2(bw.max(1.0), rect.height()));
        if active && i == cur {
            painter.rect_filled(bar.expand(2.0), 0.0, ACCENT.linear_multiply(0.18));
            painter.rect_filled(bar, 0.0, ACCENT);
        } else {
            let k = if active { 0.35 + level * 0.65 } else { 0.25 };
            painter.rect_filled(bar, 0.0, ACCENT_DIM.linear_multiply(k));
        }
    }

    // Счётчик шага под лентой
    painter.text(
        pos2(rect.right(), rect.bottom() + 2.0),
        Align2::RIGHT_TOP,
        if active {
            format!("{:02}/{STEPS}", cur + 1)
        } else {
            "ВЫКЛ".to_string()
        },
        mono(8.5),
        if active { ACCENT } else { MUTED },
    );
}

// ---------------------------------------------------------------- окно
pub(crate) fn create(
    params: Arc<PostAmbientParams>,
    gate_level: Arc<AtomicF32>,
    break_step: Arc<AtomicU32>,
    sample_shared: Arc<sample::SampleShared>,
) -> Option<Box<dyn Editor>> {
    let egui_state = params.editor_state.clone();
    let gate_for_ui = gate_level.clone();
    let step_for_ui = break_step.clone();
    let sample_for_ui = sample_shared.clone();

    create_egui_editor(
        params.editor_state.clone(),
        (),
        |egui_ctx, _| {
            // Шрифты: Oswald + JetBrains Mono (кириллица есть в обоих)
            let mut fonts = FontDefinitions::default();
            fonts.font_data.insert(
                "display".into(),
                Arc::new(FontData::from_static(include_bytes!(
                    "../fonts/Display.ttf"
                ))),
            );
            fonts.font_data.insert(
                "mono".into(),
                Arc::new(FontData::from_static(include_bytes!(
                    "../fonts/Mono-Regular.ttf"
                ))),
            );
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
            fonts
                .families
                .insert(FontFamily::Monospace, vec!["mono".into()]);
            fonts.families.insert(
                FontFamily::Name("mono_bold".into()),
                vec!["mono_bold".into()],
            );
            egui_ctx.set_fonts(fonts);

            // Стиль «минимализм»
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

                    // ================= ШАПКА =================
                    let (head, painter) =
                        ui.allocate_painter(vec2(ui.available_width(), 58.0), Sense::hover());
                    let hr = head.rect;
                    painter.rect_filled(hr, 0.0, PANEL);
                    painter.text(
                        pos2(hr.left(), hr.top() + 4.0),
                        Align2::LEFT_TOP,
                        "ДЖАНГЛ ЭМБИЕНТ",
                        disp(25.0),
                        TEXT,
                    );
                    painter.text(
                        pos2(hr.left() + 1.0, hr.top() + 37.0),
                        Align2::LEFT_TOP,
                        "AMEN BREAK · SUB BASS · AMBIENT",
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

                    // MIDI-LED: горит пока зажата нота
                    let gate = gate_for_ui.load(Ordering::Relaxed);
                    let midi_led = Rect::from_center_size(
                        pos2(hr.right() - 84.0, hr.top() + 12.0),
                        vec2(9.0, 9.0),
                    );
                    if gate > 0.01 {
                        painter.rect_filled(midi_led.expand(3.5), 0.0, LED_ON.linear_multiply(0.25));
                        painter.rect_filled(midi_led, 0.0, LED_ON);
                    } else {
                        painter.rect_filled(midi_led, 0.0, LINE);
                    }
                    painter.text(
                        pos2(midi_led.left() - 6.0, midi_led.center().y),
                        Align2::RIGHT_CENTER,
                        "MIDI",
                        monob(8.5),
                        if gate > 0.01 { LED_ON } else { MUTED },
                    );

                    painter.rect_filled(
                        Rect::from_min_size(pos2(hr.left(), hr.bottom() - 1.0), vec2(hr.width(), 1.0)),
                        0.0,
                        LINE,
                    );

                    ui.add_space(6.0);

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            // ============ 01 ДОРОЖКИ ============
                            panel(ui, |ui| {
                                section_title(ui, "01", "ДОРОЖКИ");
                                ui.horizontal(|ui| {
                                    let w = (ui.available_width() - 12.0) / 3.0;
                                    toggle_row(
                                        ui,
                                        setter,
                                        &params.break_on,
                                        "БРЕЙК",
                                        ACCENT,
                                        w,
                                    );
                                    toggle_row(
                                        ui,
                                        setter,
                                        &params.bass_on,
                                        "БАС",
                                        ACCENT,
                                        w,
                                    );
                                    toggle_row(
                                        ui,
                                        setter,
                                        &params.pad_on,
                                        "ПАД",
                                        ACCENT,
                                        w,
                                    );
                                });
                                // Режим гейта
                                ui.horizontal(|ui| {
                                    let on = params.note_gate.value();
                                    if chip(
                                        ui,
                                        if on { "ГЕЙТ: ПО НОТАМ" } else { "ГЕЙТ: ВСЕГДА" },
                                        on,
                                        vec2(112.0, 18.0),
                                    )
                                    .clicked()
                                    {
                                        setter.begin_set_parameter(&params.note_gate);
                                        setter.set_parameter(&params.note_gate, !on);
                                        setter.end_set_parameter(&params.note_gate);
                                    }
                                    ui.label(
                                        RichText::new("бас и пад — пока зажаты ноты")
                                            .font(mono(8.5))
                                            .color(MUTED),
                                    );
                                });
                            });
                            ui.add_space(5.0);

                            // ============ 02 БРЕЙК ============
                            panel(ui, |ui| {
                                section_title(ui, "02", "БРЕЙК");
                                let mode = params.break_mode.value();
                                let is_file = mode == BreakMode::File;

                                // Режим: встроенный амен-паттерн ИЛИ свой файл
                                ui.horizontal(|ui| {
                                    if chip(ui, "ИСТРОЕННЫЙ", !is_file, vec2(96.0, 19.0))
                                        .clicked()
                                    {
                                        set_param(setter, &params.break_mode, BreakMode::BuiltIn);
                                    }
                                    if chip(ui, "СВОЙ ФАЙЛ", is_file, vec2(86.0, 19.0)).clicked()
                                    {
                                        set_param(setter, &params.break_mode, BreakMode::File);
                                    }
                                    ui.label(
                                        RichText::new("32 шага · 2 такта по 16-х")
                                            .font(mono(8.5))
                                            .color(MUTED),
                                    );
                                });
                                ui.add_space(3.0);

                                // Лента шагов
                                let level = params.break_level.value();
                                step_strip(
                                    ui,
                                    &step_for_ui,
                                    level,
                                    params.break_on.value() && !is_file,
                                );
                                ui.add_space(6.0);

                                ui.columns(2, |cols| {
                                    slider_row(&mut cols[0], setter, &params.break_level, "УРОВЕНЬ");
                                    slider_row(&mut cols[1], setter, &params.swing, "СВИНГ");
                                });

                                // Свой файл: кнопка загрузки + имя + подсказка
                                if is_file {
                                    ui.add_space(3.0);
                                    let (name, is_loading, err_msg) = {
                                        let g = sample::lock(&sample_for_ui.loaded);
                                        let name = g.as_ref().map(|s| s.name.clone());
                                        let (is_loading, err) = {
                                            let st = sample::lock(&sample_for_ui.status);
                                            match &*st {
                                                UiStatus::Idle => (false, None),
                                                UiStatus::Loading => (true, None),
                                                UiStatus::Error(e) => (false, Some(e.clone())),
                                            }
                                        };
                                        (name, is_loading, err)
                                    };

                                    ui.horizontal(|ui| {
                                        if chip(ui, "ОТКРЫТЬ ФАЙЛ…", false, vec2(106.0, 20.0))
                                            .clicked()
                                        {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .add_filter(
                                                    "Аудио",
                                                    &[
                                                        "wav", "mp3", "flac", "ogg", "aiff", "aif",
                                                        "m4a",
                                                    ],
                                                )
                                                .pick_file()
                                            {
                                                sample::spawn_load(path, sample_for_ui.clone());
                                            }
                                        }
                                        let text = if is_loading {
                                            RichText::new("ДЕКОДИРУЮ…")
                                                .font(mono(9.0))
                                                .color(ACCENT)
                                        } else if let Some(n) = &name {
                                            let shown = if n.chars().count() > 24 {
                                                let t: String = n.chars().take(22).collect();
                                                format!("{t}…")
                                            } else {
                                                n.clone()
                                            };
                                            RichText::new(shown).font(monob(9.5)).color(TEXT)
                                        } else {
                                            RichText::new("— ФАЙЛ НЕ ЗАГРУЖЕН —")
                                                .font(mono(9.0))
                                                .color(MUTED)
                                        };
                                        ui.label(text);
                                    });
                                    if let Some(e) = err_msg {
                                        ui.label(
                                            RichText::new(format!("ОШИБКА: {e}"))
                                                .font(mono(8.5))
                                                .color(ERR),
                                        );
                                    }
                                    ui.label(
                                        RichText::new("ноты C2.. = 16 слайсов · октавы повторяют")
                                            .font(mono(8.5))
                                            .color(MUTED),
                                    );
                                } else {
                                    ui.label(
                                        RichText::new("играет по BPM сам, ноты не нужны")
                                            .font(mono(8.5))
                                            .color(MUTED),
                                    );
                                }
                            });
                            ui.add_space(5.0);

                            // ============ 03 СУБ-БАС ============
                            panel(ui, |ui| {
                                section_title(ui, "03", "СУБ-БАС");
                                ui.columns(2, |cols| {
                                    slider_row(&mut cols[0], setter, &params.bass_tune, "ТЕЙН (ПОЛУТОНА)");
                                    slider_row(&mut cols[1], setter, &params.drop, "ДРОП (808)");
                                });
                                slider_row(ui, setter, &params.glide, "СЛАЙД");
                            });
                            ui.add_space(5.0);

                            // ============ 04 ЭМБИЕНТ ============
                            panel(ui, |ui| {
                                section_title(ui, "04", "ЭМБИЕНТ");
                                ui.columns(2, |cols| {
                                    slider_row(&mut cols[0], setter, &params.filter_cutoff, "СРЕЗ");
                                    slider_row(
                                        &mut cols[1],
                                        setter,
                                        &params.resonance,
                                        "РЕЗОНАНС",
                                    );
                                });
                                ui.columns(2, |cols| {
                                    slider_row(&mut cols[0], setter, &params.modulation, "МОДУЛЯЦИЯ");
                                    slider_row(&mut cols[1], setter, &params.distortion, "ДИСТОРШН");
                                });
                                ui.columns(2, |cols| {
                                    slider_row(&mut cols[0], setter, &params.echo_delay, "ЭХО");
                                    slider_row(&mut cols[1], setter, &params.reverb_size, "РЕВЕРБ");
                                });
                                slider_row(ui, setter, &params.stereo_width, "СТЕРЕО");
                            });
                            ui.add_space(5.0);

                            // ============ 05 ТЕМП ============
                            panel(ui, |ui| {
                                section_title(ui, "05", "ТЕМП");
                                ui.horizontal(|ui| {
                                    let sync = params.tempo_sync.value();
                                    if chip(
                                        ui,
                                        if sync { "СИНК: ХОСТ" } else { "СИНК: РУЧН" },
                                        sync,
                                        vec2(100.0, 20.0),
                                    )
                                    .clicked()
                                    {
                                        set_param(setter, &params.tempo_sync, !sync);
                                    }
                                    ui.label(
                                        RichText::new("ведёт брейк и эхо (восьмая от BPM)")
                                            .font(mono(8.5))
                                            .color(MUTED),
                                    );
                                });
                                slider_row(ui, setter, &params.tempo, "ТЕМП (BPM)");
                            });
                            ui.add_space(5.0);

                            // ============ 06 ВЫХОД ============
                            panel(ui, |ui| {
                                section_title(ui, "06", "ВЫХОД");
                                slider_row(ui, setter, &params.output, "ГРОМКОСТЬ");
                            });
                            ui.add_space(5.0);

                            // ============ 07 СЦЕНЫ (пресеты) ============
                            panel(ui, |ui| {
                                section_title(ui, "07", "СЦЕНЫ");

                                // ДЖАНГЛ 174: классика - быстрый брейк, суб, короткое эхо
                                if chip(ui, "ДЖАНГЛ 174", false, vec2(104.0, 20.0)).clicked() {
                                    set_param(setter, &params.break_on, true);
                                    set_param(setter, &params.bass_on, true);
                                    set_param(setter, &params.pad_on, true);
                                    set_param(setter, &params.break_mode, BreakMode::BuiltIn);
                                    set_param(setter, &params.break_level, 0.85);
                                    set_param(setter, &params.swing, 0.2);
                                    set_param(setter, &params.tempo, 174.0);
                                    set_param(setter, &params.tempo_sync, true);
                                    set_param(setter, &params.bass_tune, -12);
                                    set_param(setter, &params.drop, 0.5);
                                    set_param(setter, &params.glide, 0.1);
                                    set_param(setter, &params.distortion, 0.3);
                                    set_param(setter, &params.filter_cutoff, 1600.0);
                                    set_param(setter, &params.resonance, 0.35);
                                    set_param(setter, &params.modulation, 0.2);
                                    set_param(setter, &params.reverb_size, 0.7);
                                    set_param(setter, &params.stereo_width, 0.6);
                                }

                                // АМЕН 160: жёстче и злее, больше свинга
                                if chip(ui, "АМЕН 160", false, vec2(96.0, 20.0)).clicked() {
                                    set_param(setter, &params.break_on, true);
                                    set_param(setter, &params.bass_on, true);
                                    set_param(setter, &params.pad_on, false);
                                    set_param(setter, &params.break_mode, BreakMode::BuiltIn);
                                    set_param(setter, &params.break_level, 0.95);
                                    set_param(setter, &params.swing, 0.35);
                                    set_param(setter, &params.tempo, 160.0);
                                    set_param(setter, &params.tempo_sync, true);
                                    set_param(setter, &params.bass_tune, -12);
                                    set_param(setter, &params.drop, 0.65);
                                    set_param(setter, &params.glide, 0.05);
                                    set_param(setter, &params.distortion, 0.5);
                                    set_param(setter, &params.filter_cutoff, 2400.0);
                                    set_param(setter, &params.resonance, 0.5);
                                    set_param(setter, &params.modulation, 0.15);
                                    set_param(setter, &params.reverb_size, 0.55);
                                    set_param(setter, &params.stereo_width, 0.5);
                                }

                                // ЭМБИЕНТ НОЧЬ: медленно, глубоко, без брейка
                                if chip(ui, "ЭМБИЕНТ НОЧЬ", false, vec2(116.0, 20.0)).clicked() {
                                    set_param(setter, &params.break_on, false);
                                    set_param(setter, &params.bass_on, true);
                                    set_param(setter, &params.pad_on, true);
                                    set_param(setter, &params.tempo, 90.0);
                                    set_param(setter, &params.tempo_sync, false);
                                    set_param(setter, &params.bass_tune, -12);
                                    set_param(setter, &params.drop, 0.1);
                                    set_param(setter, &params.glide, 0.5);
                                    set_param(setter, &params.distortion, 0.1);
                                    set_param(setter, &params.filter_cutoff, 700.0);
                                    set_param(setter, &params.resonance, 0.3);
                                    set_param(setter, &params.modulation, 0.6);
                                    set_param(setter, &params.echo_delay, 1.2);
                                    set_param(setter, &params.reverb_size, 0.95);
                                    set_param(setter, &params.stereo_width, 0.9);
                                }

                                ui.label(
                                    RichText::new(
                                        "174 — джангл · 160 — тяжёлый амен · ночь — эмбиент",
                                    )
                                    .font(mono(8.5))
                                    .color(MUTED),
                                );
                            });

                            // ============ ПОДВАЛ ============
                            ui.add_space(6.0);
                            let (foot, painter) = ui.allocate_painter(
                                vec2(ui.available_width(), 20.0),
                                Sense::hover(),
                            );
                            let fr = foot.rect;
                            painter.rect_filled(
                                Rect::from_min_size(pos2(fr.left(), fr.top()), vec2(fr.width(), 1.0)),
                                0.0,
                                LINE,
                            );
                            painter.text(
                                pos2(fr.left(), fr.center().y + 4.0),
                                Align2::LEFT_CENTER,
                                "БРЕЙК ИГРАЕТ САМ · БАС И ПАД — ПО НОТАМ",
                                mono(8.5),
                                ACCENT,
                            );
                            painter.text(
                                pos2(fr.right(), fr.center().y + 4.0),
                                Align2::RIGHT_CENTER,
                                "БРЕЙК · СУБ · ПАД",
                                mono(8.5),
                                MUTED,
                            );
                        });
                });
        },
    )
}
