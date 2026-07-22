//! Fixed application parameters and the authoritative harvester CSV schema.
//!
//! Keeping these values in one module makes the assumptions visible and gives
//! later hardware/logger revisions one obvious place to update the display.

use eframe::egui::Color32;

/// Native window title and visible application identity.
pub const APP_TITLE: &str = "xSAR Sensor Logger Display — Version 2";

/// Current display implementation version.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default CSV produced by the current Raspberry Pi sensor harvester.
pub const DEFAULT_CSV_PATH: &str = "/tmp/RPi_thermal_sensor_log.csv";

/// Timestamp format emitted by the harvester.
pub const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// Initial desktop window size.
pub const INITIAL_WINDOW_WIDTH: f32 = 1440.0;
pub const INITIAL_WINDOW_HEIGHT: f32 = 960.0;

/// Height of each oscilloscope-style plot group.
pub const PLOT_HEIGHT: f32 = 205.0;

/// Maximum number of simultaneously loaded runs.
///
/// This is a deliberate guard against accidentally loading a directory full of
/// long captures and turning every redraw into a large allocation/copy cycle.
pub const MAX_LOADED_RUNS: usize = 8;

/// Number of numeric columns following the timestamp in the harvester CSV.
pub const FIELD_COUNT: usize = 17;

/// Sentinel values currently written by the harvester when an NTC calculation
/// is invalid. They are retained in the raw sample but omitted from plots.
pub const INVALID_TEMPERATURE_C: f64 = -999.9;
pub const INVALID_RESISTANCE_OHM: f64 = 999_999.0;

/// Values within this tolerance of a sentinel are considered invalid.
pub const INVALID_VALUE_EPSILON: f64 = 0.000_1;

/// Column indices in the numeric sample array.
pub mod field {
    pub const VTRACK: usize = 0;
    pub const VCC_EST: usize = 1;
    pub const BMP_C: usize = 2;
    pub const BMP_PA: usize = 3;
    pub const BMP_MSLP_PA: usize = 4;
    pub const V0: usize = 5;
    pub const R0: usize = 6;
    pub const T0_BETA: usize = 7;
    pub const T0_SH: usize = 8;
    pub const V1: usize = 9;
    pub const R1: usize = 10;
    pub const T1_BETA: usize = 11;
    pub const T1_SH: usize = 12;
    pub const V2: usize = 13;
    pub const R2: usize = 14;
    pub const T2_BETA: usize = 15;
    pub const T2_SH: usize = 16;
}

/// Metadata for one harvester field.
#[derive(Clone, Copy, Debug)]
pub struct FieldSpec {
    /// Exact CSV header emitted by the harvester.
    pub header: &'static str,
    /// Compact legend label.
    pub label: &'static str,
    /// Stable channel colour across every loaded run.
    pub colour: Color32,
}

/// Authoritative CSV schema after the timestamp field.
pub const FIELDS: [FieldSpec; FIELD_COUNT] = [
    FieldSpec { header: "Vtrack", label: "Vtrack", colour: Color32::from_rgb(180, 140, 255) },
    FieldSpec { header: "VccEst", label: "Vcc estimate", colour: Color32::from_rgb(255, 190, 70) },
    FieldSpec { header: "BmpC", label: "BMP180 reference", colour: Color32::from_rgb(255, 255, 255) },
    FieldSpec { header: "BmpPa", label: "BMP180 station", colour: Color32::from_rgb(80, 190, 255) },
    FieldSpec { header: "BmpMslpPa", label: "BMP180 MSLP", colour: Color32::from_rgb(80, 255, 190) },
    FieldSpec { header: "V0", label: "ADS A0", colour: Color32::from_rgb(255, 90, 90) },
    FieldSpec { header: "R0", label: "NTC A0", colour: Color32::from_rgb(255, 90, 90) },
    FieldSpec { header: "T0", label: "A0 β", colour: Color32::from_rgb(255, 90, 90) },
    FieldSpec { header: "T0sh", label: "A0 SH", colour: Color32::from_rgb(255, 150, 150) },
    FieldSpec { header: "V1", label: "ADS A1", colour: Color32::from_rgb(100, 220, 120) },
    FieldSpec { header: "R1", label: "NTC A1", colour: Color32::from_rgb(100, 220, 120) },
    FieldSpec { header: "T1", label: "A1 β", colour: Color32::from_rgb(100, 220, 120) },
    FieldSpec { header: "T1sh", label: "A1 SH", colour: Color32::from_rgb(160, 255, 175) },
    FieldSpec { header: "V2", label: "ADS A2", colour: Color32::from_rgb(90, 150, 255) },
    FieldSpec { header: "R2", label: "NTC A2", colour: Color32::from_rgb(90, 150, 255) },
    FieldSpec { header: "T2", label: "A2 β", colour: Color32::from_rgb(90, 150, 255) },
    FieldSpec { header: "T2sh", label: "A2 SH", colour: Color32::from_rgb(150, 190, 255) },
];

/// Exact first CSV header.
pub const TIMESTAMP_HEADER: &str = "timestamp";
