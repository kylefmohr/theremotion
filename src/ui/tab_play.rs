use std::ops::RangeInclusive;

use egui::{
    plot::{
        uniform_grid_spacer, Bar, BarChart, GridMark, HLine, Legend, Line, MarkerShape, PlotPoint,
        PlotPoints, Points, VLine,
    },
    Widget,
};
use staff::midi::MidiNote;

use crate::{
    controls::{self},
    settings::Preset,
    ui::KeyboardEditMode,
};

pub struct TabPlay<'a> {
    controls: &'a mut controls::Controls,
    preset: &'a mut Preset,
}

impl<'a> TabPlay<'a> {
    pub fn new(controls: &'a mut controls::Controls, preset: &'a mut Preset) -> Self {
        Self { controls, preset }
    }
}

impl Widget for TabPlay<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.add(crate::ui::Keyboard::new(
                self.controls.lead.iter().collect(),
                self.preset,
                KeyboardEditMode::Drone,
            ));
            ui.separator();
            ui.horizontal(|ui| {
                let height = 250.0;
                self.tuner(ui, 75.0, height, "tuner");
                ui.add_space(4.0);
                self.autotune_plot(ui, height);
                ui.add_space(8.0);
                self.filter_plot(ui, height, "filter");
                ui.add_space(4.0);
                self.volume(ui, 75.0, height, "volume");
            });
        })
        .response
    }
}

impl<'a> TabPlay<'a> {
    fn autotune_plot(&self, ui: &mut egui::Ui, size: f32) {
        let note_range = self.preset.note_range();
        let scale_window = self.preset.restricted_scale_floating_window();
        let smooths =
            (*note_range.start() as usize * 10..*note_range.end() as usize * 10).map(|i| {
                let x = i as f32 * 0.1;
                PlotPoint::new(x, scale_window.autotune(x, self.controls.autotune))
            });
        let line = Line::new(PlotPoints::Owned(smooths.collect()));
        // hack: force the include_x/include_y to recenter on root note change
        let plot_id = format!("{}{}", self.preset.pitch, self.preset.octave);
        egui::plot::Plot::new(plot_id)
            .allow_boxed_zoom(false)
            .allow_drag(false)
            .allow_scroll(false)
            .allow_zoom(false)
            .include_x(*note_range.start())
            .include_x(*note_range.end())
            .include_y(*note_range.start())
            .include_y(*note_range.end())
            .x_grid_spacer(uniform_grid_spacer(|_| [12.0, 1.0, 1.0]))
            .y_grid_spacer(uniform_grid_spacer(|_| [12.0, 1.0, 1.0]))
            .x_axis_formatter(note_formatter)
            .y_axis_formatter(note_formatter)
            .legend(Legend::default())
            .width(size)
            .height(size)
            .show(ui, |plot_ui| {
                plot_ui.line(line);
                plot_ui.points(
                    Points::new(PlotPoints::Owned(vec![PlotPoint::new(
                        self.controls.raw_note,
                        self.controls.lead[0].note.value,
                    )]))
                    .shape(MarkerShape::Plus)
                    .radius(6.0),
                );
            });
    }

    fn volume(&self, ui: &mut egui::Ui, width: f32, height: f32, plot_name: &str) {
        let volume = self.controls.lead_volume.value;
        egui::plot::Plot::new(plot_name)
            .allow_boxed_zoom(false)
            .allow_drag(false)
            .allow_scroll(false)
            .allow_zoom(false)
            .include_y(0.0)
            .include_y(1.0)
            .include_x(-1.0)
            .include_x(1.0)
            .show_axes([false, false])
            .width(width)
            .height(height)
            .show(ui, |plot_ui| {
                plot_ui.bar_chart(BarChart::new(vec![Bar::new(0.0, volume.into())]).width(2.0));
            });
    }

    fn tuner(&self, ui: &mut egui::Ui, width: f32, height: f32, plot_name: &str) {
        let scale = self.preset.restricted_scale();
        let scale_window = self.preset.restricted_scale_floating_window();
        let note_raw = self.controls.raw_note;
        let note_tuned = self.controls.lead[0].note.value;
        let closest = scale_window.closest_in_scale(note_raw);

        // hack: force the include_x/include_y to recenter on root note change
        let plot_id = format!("{plot_name}{closest}");
        egui::plot::Plot::new(plot_id)
            .allow_boxed_zoom(false)
            .allow_drag(false)
            .allow_scroll(false)
            .allow_zoom(false)
            .include_y(closest - 2.0)
            .include_y(closest + 2.0)
            .include_x(-1.0)
            .include_x(1.0)
            .y_grid_spacer(move |input| {
                ((input.bounds.0.floor() as u8)..=(input.bounds.1.ceil() as u8))
                    .into_iter()
                    .map(|n| {
                        let note = MidiNote::from_byte(n);
                        let step_size = if scale.contains(&note) { 5.0 } else { 1.0 };
                        GridMark {
                            value: n as f64,
                            step_size,
                        }
                    })
                    .collect()
            })
            .show_axes([false, true])
            .y_axis_formatter(note_formatter)
            .width(width)
            .height(height)
            .show(ui, |plot_ui| {
                plot_ui.hline(HLine::new(note_raw).name("Note"));
                plot_ui.hline(HLine::new(note_tuned).name("Note (Tuned)"));
            });
    }

    fn filter_plot(&self, ui: &mut egui::Ui, size: f32, plot_name: &str) {
        let cutoff = &self.controls.cutoff_note;
        let resonance = &self.controls.resonance;
        egui::plot::Plot::new(plot_name)
            .allow_boxed_zoom(false)
            .allow_drag(false)
            .allow_scroll(false)
            .allow_zoom(false)
            .include_x(*cutoff.input.range.start())
            .include_x(*cutoff.input.range.end())
            .include_y(*resonance.input.range.start())
            .include_y(*resonance.input.range.end())
            .legend(Legend::default())
            .show_axes([false, false])
            .width(size)
            .height(size)
            .show(ui, |plot_ui| {
                plot_ui.vline(VLine::new(cutoff.value).name("Cutoff"));
                plot_ui.hline(HLine::new(resonance.value).name("Resonance"));
            });
    }
}

fn note_formatter(note: f64, _range: &RangeInclusive<f64>) -> String {
    MidiNote::from_byte(note as u8).to_string()
}
