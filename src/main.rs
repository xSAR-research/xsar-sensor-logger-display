//! xSAR Sensor Logger Display — Version 2.
//!
//! This desktop application visualises CSV files produced by the xSAR NTC batch
//! harvester. Separate captures are compared by elapsed time, so experiments
//! performed on different dates can be overlaid without falsifying their
//! wall-clock relationship.

mod app;
mod data;
mod parameters;

use app::SensorApp;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([
            parameters::INITIAL_WINDOW_WIDTH,
            parameters::INITIAL_WINDOW_HEIGHT,
        ]),
        ..Default::default()
    };

    eframe::run_native(
        parameters::APP_TITLE,
        native_options,
        Box::new(|creation_context| Ok(Box::new(SensorApp::new(creation_context)))),
    )
}
