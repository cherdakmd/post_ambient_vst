// src/editor.rs - GUI плагина (egui).
//
// Стиль по концепции README: тёмный дизайн, «брутализм», бетон и серое небо,
// холодный серо-синий акцент, минимализм.

use crate::{next_noise_type, noise_type_label, PostAmbientParams};
use nih_plug::prelude::*;
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, Color32, RichText, Vec2},
    resizable_window::ResizableWindow,
    widgets,
};
use std::sync::Arc;

/// Переключатель (булевый параметр).
macro_rules! toggle {
    ($ui:expr, $setter:expr, $param:expr, $label:expr) => {{
        let mut value = $param.value();
        if $ui
            .toggle_value(&mut value, RichText::new($label).strong())
            .changed()
        {
            $setter.begin_set_parameter($param);
            $setter.set_parameter($param, value);
            $setter.end_set_parameter($param);
        }
    }};
}

/// Подпись + слайдер параметра.
macro_rules! slider {
    ($ui:expr, $setter:expr, $param:expr, $label:expr) => {{
        $ui.label(
            RichText::new($label)
                .size(11.0)
                .color(Color32::from_rgb(150, 158, 166)),
        );
        $ui.add(widgets::ParamSlider::for_param($param, $setter));
    }};
}

fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.add_space(10.0);
    ui.label(
        RichText::new(title)
            .strong()
            .size(13.0)
            .color(Color32::from_rgb(175, 183, 191)),
    );
    ui.separator();
}

pub(crate) fn create(params: Arc<PostAmbientParams>) -> Option<Box<dyn Editor>> {
    let egui_state = params.editor_state.clone();

    create_egui_editor(
        params.editor_state.clone(),
        (),
        |_, _| {},
        move |egui_ctx, setter, _state| {
            // Стиль «бетон»: тёмный фон, холодный серо-синий акцент
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(24, 24, 24);
            visuals.window_fill = Color32::from_rgb(31, 31, 31);
            visuals.extreme_bg_color = Color32::from_rgb(48, 48, 48);
            visuals.faint_bg_color = Color32::from_rgb(37, 37, 37);
            visuals.selection.bg_fill = Color32::from_rgb(110, 125, 140);
            visuals.slider_trailing_fill = true;
            visuals.widgets.hovered.bg_stroke =
                egui::Stroke::new(1.0_f32, Color32::from_rgb(140, 150, 160));
            egui_ctx.set_visuals(visuals);

            ResizableWindow::new("post-ambient-window")
                .min_size(Vec2::new(380.0, 480.0))
                .show(egui_ctx, egui_state.as_ref(), |ui| {
                    ui.set_min_width(340.0);

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // --- Заголовок ---
                        ui.label(
                            RichText::new("POST AMBIENT VST")
                                .size(22.0)
                                .strong()
                                .color(Color32::from_rgb(215, 220, 225)),
                        );
                        ui.label(
                            RichText::new("ЮГО-ЗАПАД СПб · POST-PUNK 2000s · v2.1")
                                .size(12.0)
                                .color(Color32::from_rgb(125, 135, 145)),
                        );

                        // --- Дорожки (стемы) ---
                        section_title(ui, "ДОРОЖКИ (STEMS)");
                        ui.horizontal(|ui| {
                            toggle!(ui, setter, &params.noise_on, "NOISE");
                            toggle!(ui, setter, &params.bass_on, "BASS");
                            toggle!(ui, setter, &params.pad_on, "PAD");
                        });

                        // --- Шум ---
                        section_title(ui, "ШУМ");
                        let current_kind = params.noise_type.value();
                        if ui
                            .button(format!(
                                "ТИП: {}  ▸",
                                noise_type_label(current_kind)
                            ))
                            .clicked()
                        {
                            let next = next_noise_type(current_kind);
                            setter.begin_set_parameter(&params.noise_type);
                            setter.set_parameter(&params.noise_type, next);
                            setter.end_set_parameter(&params.noise_type);
                        }
                        slider!(ui, setter, &params.noise_level, "NOISE LEVEL");
                        slider!(ui, setter, &params.granularity, "GRANULARITY");
                        slider!(ui, setter, &params.modulation, "MODULATION");
                        slider!(ui, setter, &params.seed, "SEED (RND)");

                        // --- Бас ---
                        section_title(ui, "БАС");
                        slider!(ui, setter, &params.bass_freq, "BASS FREQ");
                        slider!(ui, setter, &params.bass_boost, "BASS BOOST");
                        slider!(ui, setter, &params.glide, "GLIDE (СЛАЙД)");
                        slider!(ui, setter, &params.distortion, "DISTORTION");

                        // --- Атмосфера ---
                        section_title(ui, "АТМОСФЕРА");
                        slider!(ui, setter, &params.filter_cutoff, "FILTER CUTOFF");
                        slider!(ui, setter, &params.resonance, "RESONANCE");
                        slider!(ui, setter, &params.echo_delay, "ECHO DELAY");
                        slider!(ui, setter, &params.reverb_size, "REVERB SIZE");
                        slider!(ui, setter, &params.stereo_width, "STEREO (ДВОРЫ)");

                        // --- Темп ---
                        section_title(ui, "ТЕМП");
                        toggle!(ui, setter, &params.tempo_sync, "SYNC TO HOST");
                        slider!(ui, setter, &params.tempo, "TEMPO (BPM)");

                        // --- Выход ---
                        section_title(ui, "ВЫХОД");
                        slider!(ui, setter, &params.output, "OUTPUT");

                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("шум · бас · пад — три дорожки, один бетон")
                                .size(11.0)
                                .italics()
                                .color(Color32::from_rgb(105, 112, 120)),
                        );
                        ui.add_space(4.0);
                    });
                });
        },
    )
}
