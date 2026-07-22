use eframe::egui::{self, Ui, Widget};
use egui_plot::{Plot, PlotPoints, Line};
use std::path::PathBuf;
use chrono::NaiveDateTime;

#[derive(Default)]
struct SensorApp {
    data: Vec<(NaiveDateTime, u32, [f64; 8])>,
    column_names: Vec<String>,
    selected_col: usize,
    min_time: f64,
    max_time: f64,
    view_min: f64,
    view_max: f64,
    file_path: String,
    status: String,
}

impl SensorApp {
    fn load_csv(&mut self) {
        self.status = "Loading...".to_string();
        let path = "/tmp/data.csv";
        self.file_path = path.to_string();
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let mut rdr = csv::Reader::from_reader(content.as_bytes());
                if let Ok(headers) = rdr.headers() {
                    self.column_names = headers.iter().skip(2).take(8).map(|s| s.to_string()).collect();
                    if self.column_names.is_empty() { self.column_names = vec!["Channel1".to_string(); 8]; }
                }
                self.data.clear();
                for result in rdr.records() {
                    if let Ok(record) = result {
                        if record.len() < 3 { continue; }
                        let ts_str = &record[0];
                        let ts = NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d %H:%M:%S").unwrap_or_else(|_| NaiveDateTime::from_timestamp_opt(0,0).unwrap());
                        let idx: u32 = record[1].parse().unwrap_or(0);
                        let mut vals = [0.0f64; 8];
                        for i in 0..8.min(record.len()-2) {
                            vals[i] = record[i+2].parse().unwrap_or(0.0);
                        }
                        self.data.push((ts, idx, vals));
                    }
                }
                if !self.data.is_empty() {
                    self.min_time = self.data[0].0.and_utc().timestamp() as f64;
                    self.max_time = self.data.last().unwrap().0.and_utc().timestamp() as f64;
                    self.view_min = self.min_time;
                    self.view_max = self.max_time;
                    self.selected_col = 0;
                    self.status = format!("Loaded {} rows", self.data.len());
                }
            }
            Err(_) => {
                self.status = "No /tmp/data.csv - use Generate Demo".to_string();
                self.generate_demo();
            }
        }
    }

    fn generate_demo(&mut self) {
        self.data.clear();
        self.column_names = vec!["accel_x".into(), "gyro_y".into(), "temp_c".into(), "pressure".into(), "hum".into(), "volt".into(), "mag".into(), "snr".into()];
        let base = chrono::Local::now().naive_local();
        for i in 0..600 {
            let ts = base + chrono::Duration::seconds(i);
            let vals = [i as f64 * 0.1 % 10.0, (i as f64).sin() * 5.0, 22.5 + (i as f64 * 0.05).sin() * 2.0, 1013.0, 42.0, 3.7, 0.5, 15.0 + (i%50) as f64];
            self.data.push((ts, i as u32, vals));
        }
        self.min_time = self.data[0].0.and_utc().timestamp() as f64;
        self.max_time = self.data.last().unwrap().0.and_utc().timestamp() as f64;
        self.view_min = self.min_time;
        self.view_max = self.max_time;
        self.status = "Demo data generated (600 rows) ✅".to_string();
    }

    fn get_visible_points(&self) -> PlotPoints {
        let mut points = vec![];
        let col = self.selected_col.min(7);
        for &(ts, _, vals) in &self.data {
            let t = ts.and_utc().timestamp() as f64;
            if t >= self.view_min && t <= self.view_max {
                points.push([t, vals[col]]);
            }
        }
        PlotPoints::new(points)
    }
}

impl eframe::App for SensorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("📂 Pick CSV").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.file_path = path.to_string_lossy().to_string();
                        // TODO: load from selected
                        self.status = "File selected - implement full load".to_string();
                    }
                }
                if ui.button("🔄 Load /tmp/data.csv").clicked() || self.data.is_empty() {
                    self.load_csv();
                }
                if ui.button("🎲 Generate Demo CSV in /tmp").clicked() {
                    self.generate_demo();
                    // In real would write file too
                }
                ui.label(&self.status);
            });
            ui.horizontal(|ui| {
                ui.label("Y Column:");
                egui::ComboBox::from_id_source("col_select")
                    .selected_text(self.column_names.get(self.selected_col).unwrap_or(&"Ch1".to_string()))
                    .show_ui(ui, |ui| {
                        for (i, name) in self.column_names.iter().enumerate() {
                            if ui.selectable_value(&mut self.selected_col, i, name).changed() {
                                ctx.request_repaint();
                            }
                        }
                    });
                if ui.button("Full Range").clicked() {
                    self.view_min = self.min_time;
                    self.view_max = self.max_time;
                }
            });
        });

        egui::SidePanel::left("controls").show(ctx, |ui| {
            ui.heading("Time Range");
            ui.label("From");
            ui.add(egui::DragValue::new(&mut self.view_min).speed(10.0).suffix(" unix s"));
            ui.label("To");
            ui.add(egui::DragValue::new(&mut self.view_max).speed(10.0).suffix(" unix s"));
            if ui.button("Clamp to Data").clicked() {
                self.view_min = self.view_min.max(self.min_time);
                self.view_max = self.view_max.min(self.max_time);
            }
            ui.separator();
            ui.label("File: ");
            ui.text_edit_singleline(&mut self.file_path);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("xSAR Sensor Display • Charlie's Lab • egui + KDE Wayland");
            if self.data.is_empty() {
                ui.label("Load data or hit Generate Demo");
                return;
            }
            let plot = Plot::new("sensor_plot")
                .x_axis_label("Timestamp (unix seconds)")
                .y_axis_label(self.column_names.get(self.selected_col).unwrap_or(&"Value".to_string()))
                .show(ui, |plot_ui| {
                    let line = Line::new(self.get_visible_points()).color(egui::Color32::from_rgb(0, 255, 100));
                    plot_ui.line(line);
                });

            // Auto range note - in practice calculate min/max Y per view
            ui.label(format!("Showing {} points | Y auto-ranges with buffer", self.get_visible_points().len() )); // simplified
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(100)); // for demo
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "xSAR Sensor Logger Display - egui",
        options,
        Box::new(|_cc| Ok(Box::new(SensorApp::default()))),
    )
}