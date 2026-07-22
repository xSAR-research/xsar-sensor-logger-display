//! egui application state and oscilloscope-style presentation.

use std::path::Path;

use eframe::egui::{self, Color32, RichText};
use egui_plot::{Legend, Line, Plot, PlotPoints};

use crate::data::{load_csv, SensorRun, TimeWindow};
use crate::parameters::{self, field, FIELDS};

/// Which provisional NTC temperature calculation should be compared with the
/// BMP180 reference channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NtcModel {
    Beta,
    SteinhartHart,
    Both,
}

impl NtcModel {
    fn label(self) -> &'static str {
        match self {
            Self::Beta => "β model",
            Self::SteinhartHart => "Steinhart–Hart",
            Self::Both => "Both models",
        }
    }
}

/// Visibility state for each stacked trace group.
#[derive(Clone, Copy, Debug)]
struct GroupVisibility {
    temperatures: bool,
    temperature_delta: bool,
    voltages: bool,
    resistances: bool,
    pressure: bool,
}

impl Default for GroupVisibility {
    fn default() -> Self {
        Self {
            temperatures: true,
            temperature_delta: true,
            voltages: true,
            resistances: false,
            pressure: false,
        }
    }
}

pub struct SensorApp {
    runs: Vec<SensorRun>,
    selected_run: Option<usize>,
    time_window: TimeWindow,
    ntc_model: NtcModel,
    groups: GroupVisibility,
    status: String,
}

impl Default for SensorApp {
    fn default() -> Self {
        Self {
            runs: Vec::new(),
            selected_run: None,
            time_window: TimeWindow::default(),
            ntc_model: NtcModel::Beta,
            groups: GroupVisibility::default(),
            status: format!(
                "Load the harvester CSV at {} or add another run for comparison.",
                parameters::DEFAULT_CSV_PATH
            ),
        }
    }
}

impl SensorApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        creation_context.egui_ctx.set_visuals(egui::Visuals::dark());
        Self::default()
    }

    fn add_run(&mut self, path: &Path) {
        if self.runs.len() >= parameters::MAX_LOADED_RUNS {
            self.status = format!(
                "Run limit reached ({}). Remove a run before loading another.",
                parameters::MAX_LOADED_RUNS
            );
            return;
        }

        match load_csv(path) {
            Ok(report) => {
                let skipped = report.skipped_rows;
                let sample_count = report.run.samples.len();
                let name = report.run.display_name.clone();
                self.runs.push(report.run);
                self.selected_run = Some(self.runs.len() - 1);
                self.reset_time_window();
                self.status = if skipped == 0 {
                    format!("Loaded {name}: {sample_count} samples.")
                } else {
                    format!(
                        "Loaded {name}: {sample_count} valid samples; skipped {skipped} malformed rows."
                    )
                };
            }
            Err(error) => {
                self.status = format!("Could not load {}: {error:#}", path.display());
            }
        }
    }

    fn maximum_duration(&self) -> f64 {
        self.runs
            .iter()
            .map(SensorRun::duration_seconds)
            .fold(0.0, f64::max)
    }

    fn reset_time_window(&mut self) {
        self.time_window = TimeWindow {
            start_seconds: 0.0,
            end_seconds: self.maximum_duration(),
        };
    }

    fn selected_run_name(&self) -> &str {
        self.selected_run
            .and_then(|index| self.runs.get(index))
            .map_or("No run selected", |run| run.display_name.as_str())
    }

    fn top_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Load default CSV").clicked() {
                    self.add_run(Path::new(parameters::DEFAULT_CSV_PATH));
                }

                if ui.button("Add CSV run…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CSV sensor log", &["csv"])
                        .pick_file()
                    {
                        self.add_run(&path);
                    }
                }

                if ui.button("Remove selected").clicked() {
                    if let Some(index) = self.selected_run.take() {
                        if index < self.runs.len() {
                            let removed = self.runs.remove(index);
                            self.selected_run = (!self.runs.is_empty()).then_some(0);
                            self.reset_time_window();
                            self.status = format!("Removed {}.", removed.display_name);
                        }
                    }
                }

                if ui.button("Clear all").clicked() {
                    self.runs.clear();
                    self.selected_run = None;
                    self.time_window = TimeWindow::default();
                    self.status = "Cleared all loaded runs.".to_owned();
                }

                ui.separator();
                ui.label(RichText::new(&self.status).small());
            });
        });
    }

    fn side_controls(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("controls")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Version 2 controls");
                ui.label(format!("Display v{}", parameters::APP_VERSION));
                ui.separator();

                ui.strong("Loaded runs");
                if self.runs.is_empty() {
                    ui.label("No CSV runs loaded.");
                } else {
                    let selected_run_name = self.selected_run_name().to_owned();
                    egui::ComboBox::from_id_source("selected_run")
                        .selected_text(selected_run_name)
                        .show_ui(ui, |ui| {
                            for (index, run) in self.runs.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.selected_run,
                                    Some(index),
                                    &run.display_name,
                                );
                            }
                        });

                    if let Some(run) = self.selected_run.and_then(|index| self.runs.get(index)) {
                        ui.monospace(run.source_path.display().to_string());
                        ui.label(format!("Samples: {}", run.samples.len()));
                        ui.label(format!("Started: {}", run.started_at));
                        ui.label(format!("Ended: {}", run.ended_at));
                        ui.label(format!("Duration: {:.0} s", run.duration_seconds()));
                    }
                }

                ui.separator();
                ui.strong("Elapsed-time window");
                let maximum = self.maximum_duration();
                ui.horizontal(|ui| {
                    ui.label("From");
                    ui.add(
                        egui::DragValue::new(&mut self.time_window.start_seconds)
                            .speed(1.0)
                            .suffix(" s"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("To");
                    ui.add(
                        egui::DragValue::new(&mut self.time_window.end_seconds)
                            .speed(1.0)
                            .suffix(" s"),
                    );
                });
                self.time_window.normalise(maximum);
                if ui.button("Full elapsed range").clicked() {
                    self.reset_time_window();
                }

                ui.separator();
                ui.strong("NTC sanity-check model");
                egui::ComboBox::from_id_source("ntc_model")
                    .selected_text(self.ntc_model.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.ntc_model, NtcModel::Beta, "β model");
                        ui.selectable_value(
                            &mut self.ntc_model,
                            NtcModel::SteinhartHart,
                            "Steinhart–Hart",
                        );
                        ui.selectable_value(&mut self.ntc_model, NtcModel::Both, "Both models");
                    });
                ui.label(
                    RichText::new(
                        "BMP180 temperature is the reference. NTC temperatures are provisional divider-health traces.",
                    )
                    .small()
                    .color(Color32::LIGHT_GRAY),
                );

                ui.separator();
                ui.strong("Trace groups");
                ui.checkbox(&mut self.groups.temperatures, "Temperature comparison");
                ui.checkbox(&mut self.groups.temperature_delta, "NTC minus BMP180");
                ui.checkbox(&mut self.groups.voltages, "ADC and rail voltages");
                ui.checkbox(&mut self.groups.resistances, "Calculated NTC resistance");
                ui.checkbox(&mut self.groups.pressure, "BMP180 pressure");
            });
    }

    fn central_plots(&self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(parameters::APP_TITLE);
            ui.label(
                "Runs are aligned at elapsed time 0 s. Calendar dates are provenance only and are never used to overlay captures.",
            );
            ui.separator();

            if self.runs.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("Load one or more xSAR harvester CSV files to begin.");
                });
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.groups.temperatures {
                    self.temperature_plot(ui);
                }
                if self.groups.temperature_delta {
                    self.temperature_delta_plot(ui);
                }
                if self.groups.voltages {
                    self.raw_field_plot(
                        ui,
                        "voltage_plot",
                        "ADC and rail voltages",
                        "V",
                        &[field::VTRACK, field::VCC_EST, field::V0, field::V1, field::V2],
                    );
                }
                if self.groups.resistances {
                    self.raw_field_plot(
                        ui,
                        "resistance_plot",
                        "Calculated NTC resistance",
                        "Ω",
                        &[field::R0, field::R1, field::R2],
                    );
                }
                if self.groups.pressure {
                    self.raw_field_plot(
                        ui,
                        "pressure_plot",
                        "BMP180 pressure",
                        "Pa",
                        &[field::BMP_PA, field::BMP_MSLP_PA],
                    );
                }
            });
        });
    }

    fn temperature_fields(&self) -> Vec<usize> {
        let mut fields = vec![field::BMP_C];
        match self.ntc_model {
            NtcModel::Beta => fields.extend([field::T0_BETA, field::T1_BETA, field::T2_BETA]),
            NtcModel::SteinhartHart => {
                fields.extend([field::T0_SH, field::T1_SH, field::T2_SH])
            }
            NtcModel::Both => fields.extend([
                field::T0_BETA,
                field::T0_SH,
                field::T1_BETA,
                field::T1_SH,
                field::T2_BETA,
                field::T2_SH,
            ]),
        }
        fields
    }

    fn ntc_temperature_fields(&self) -> Vec<usize> {
        self.temperature_fields()
            .into_iter()
            .filter(|field_index| *field_index != field::BMP_C)
            .collect()
    }

    fn temperature_plot(&self, ui: &mut egui::Ui) {
        self.raw_field_plot(
            ui,
            "temperature_plot",
            "Temperature comparison — BMP180 is reference",
            "°C",
            &self.temperature_fields(),
        );
    }

    fn temperature_delta_plot(&self, ui: &mut egui::Ui) {
        ui.strong("NTC residuals relative to BMP180 reference (°C)");
        Plot::new("temperature_delta_plot")
            .height(parameters::PLOT_HEIGHT)
            .x_axis_label("Elapsed time (s)")
            .y_axis_label("Δ°C")
            .legend(Legend::default())
            .show(ui, |plot_ui| {
                for (run_index, run) in self.runs.iter().enumerate() {
                    for field_index in self.ntc_temperature_fields() {
                        let points = run.temperature_delta_points(field_index, self.time_window);
                        if points.is_empty() {
                            continue;
                        }
                        let spec = FIELDS[field_index];
                        plot_ui.line(
                            Line::new(PlotPoints::new(points))
                                .name(format!(
                                    "Run {} · {} · {} − BMP",
                                    run_index + 1,
                                    run.display_name,
                                    spec.label
                                ))
                                .color(colour_for_run(spec.colour, run_index)),
                        );
                    }
                }
            });
        ui.separator();
    }

    fn raw_field_plot(
        &self,
        ui: &mut egui::Ui,
        plot_id: &'static str,
        title: &str,
        unit: &str,
        field_indices: &[usize],
    ) {
        ui.strong(format!("{title} ({unit})"));
        Plot::new(plot_id)
            .height(parameters::PLOT_HEIGHT)
            .x_axis_label("Elapsed time (s)")
            .y_axis_label(unit)
            .legend(Legend::default())
            .show(ui, |plot_ui| {
                for (run_index, run) in self.runs.iter().enumerate() {
                    for &field_index in field_indices {
                        let points = run.field_points(field_index, self.time_window);
                        if points.is_empty() {
                            continue;
                        }
                        let spec = FIELDS[field_index];
                        plot_ui.line(
                            Line::new(PlotPoints::new(points))
                                .name(format!(
                                    "Run {} · {} · {}",
                                    run_index + 1,
                                    run.display_name,
                                    spec.label
                                ))
                                .color(colour_for_run(spec.colour, run_index)),
                        );
                    }
                }
            });
        ui.separator();
    }
}

impl eframe::App for SensorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.top_toolbar(ctx);
        self.side_controls(ctx);
        self.central_plots(ctx);
    }
}

/// Retain channel identity while making repeated runs visually distinguishable.
fn colour_for_run(base: Color32, run_index: usize) -> Color32 {
    const ALPHAS: [u8; 4] = [255, 205, 165, 125];
    let alpha = ALPHAS[run_index % ALPHAS.len()];
    Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), alpha)
}
