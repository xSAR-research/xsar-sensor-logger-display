//! CSV ingestion and run-relative data representation.
//!
//! Calendar timestamps remain available for provenance, but plots use elapsed
//! seconds from each file's first valid record. This allows captures from
//! different dates to be compared as repeatable experiments rather than as one
//! continuous wall-clock timeline.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use chrono::NaiveDateTime;

use crate::parameters::{
    self, field, FIELDS, FIELD_COUNT, INVALID_RESISTANCE_OHM, INVALID_TEMPERATURE_C,
};

/// One valid harvester row.
#[derive(Clone, Debug)]
pub struct Sample {
    /// Seconds since this run's first valid sample.
    pub elapsed_seconds: f64,
    /// Numeric values indexed through `parameters::field`.
    pub values: [f64; FIELD_COUNT],
}

/// One independently captured CSV run.
#[derive(Clone, Debug)]
pub struct SensorRun {
    pub source_path: PathBuf,
    pub display_name: String,
    pub started_at: NaiveDateTime,
    pub ended_at: NaiveDateTime,
    pub samples: Vec<Sample>,
}

impl SensorRun {
    pub fn duration_seconds(&self) -> f64 {
        self.samples
            .last()
            .map_or(0.0, |sample| sample.elapsed_seconds)
    }

    /// Return display-safe points for one raw field within the selected window.
    pub fn field_points(&self, field_index: usize, window: TimeWindow) -> Vec<[f64; 2]> {
        self.samples
            .iter()
            .filter_map(|sample| {
                let value = sample.values[field_index];
                (window.contains(sample.elapsed_seconds) && is_displayable(field_index, value))
                    .then_some([sample.elapsed_seconds, value])
            })
            .collect()
    }

    /// Return NTC temperature residuals relative to the BMP180 reference.
    pub fn temperature_delta_points(
        &self,
        ntc_temperature_field: usize,
        window: TimeWindow,
    ) -> Vec<[f64; 2]> {
        self.samples
            .iter()
            .filter_map(|sample| {
                let ntc = sample.values[ntc_temperature_field];
                let reference = sample.values[field::BMP_C];
                (window.contains(sample.elapsed_seconds)
                    && is_displayable(ntc_temperature_field, ntc)
                    && reference.is_finite())
                .then_some([sample.elapsed_seconds, ntc - reference])
            })
            .collect()
    }
}

/// Shared x-axis window in run-relative seconds.
#[derive(Clone, Copy, Debug, Default)]
pub struct TimeWindow {
    pub start_seconds: f64,
    pub end_seconds: f64,
}

impl TimeWindow {
    pub fn contains(self, elapsed_seconds: f64) -> bool {
        elapsed_seconds >= self.start_seconds && elapsed_seconds <= self.end_seconds
    }

    pub fn normalise(&mut self, maximum: f64) {
        self.start_seconds = self.start_seconds.clamp(0.0, maximum);
        self.end_seconds = self.end_seconds.clamp(0.0, maximum);
        if self.start_seconds > self.end_seconds {
            std::mem::swap(&mut self.start_seconds, &mut self.end_seconds);
        }
    }
}

/// Loader result including recoverable row errors.
#[derive(Debug)]
pub struct LoadReport {
    pub run: SensorRun,
    pub skipped_rows: usize,
}

pub fn load_csv(path: &Path) -> Result<LoadReport> {
    let file =
        File::open(path).with_context(|| format!("could not open CSV {}", path.display()))?;
    parse_reader(file, path.to_path_buf())
}

fn parse_reader<R: Read>(reader: R, source_path: PathBuf) -> Result<LoadReport> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);

    let headers = csv_reader
        .headers()
        .context("could not read CSV header")?
        .clone();
    let column_map = build_column_map(&headers)?;

    let mut rows = Vec::new();
    let mut skipped_rows = 0;

    for record_result in csv_reader.records() {
        let record = match record_result {
            Ok(record) => record,
            Err(_) => {
                skipped_rows += 1;
                continue;
            }
        };

        match parse_record(&record, &column_map) {
            Ok(row) => rows.push(row),
            Err(_) => skipped_rows += 1,
        }
    }

    if rows.is_empty() {
        bail!("CSV contains no valid sensor rows");
    }

    // Preserve logger order. The elapsed axis is intentionally based on the
    // first valid sample, not on the filename date or another loaded run.
    let started_at = rows[0].0;
    let ended_at = rows[rows.len() - 1].0;
    let samples = rows
        .into_iter()
        .map(|(timestamp, values)| Sample {
            elapsed_seconds: (timestamp - started_at).num_milliseconds() as f64 / 1000.0,
            values,
        })
        .collect();

    let display_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("sensor run")
        .to_owned();

    Ok(LoadReport {
        run: SensorRun {
            source_path,
            display_name,
            started_at,
            ended_at,
            samples,
        },
        skipped_rows,
    })
}

fn build_column_map(headers: &csv::StringRecord) -> Result<HashMap<&'static str, usize>> {
    let mut map = HashMap::with_capacity(FIELD_COUNT + 1);

    let timestamp_index = headers
        .iter()
        .position(|header| header == parameters::TIMESTAMP_HEADER)
        .ok_or_else(|| anyhow!("missing required '{}' column", parameters::TIMESTAMP_HEADER))?;
    map.insert(parameters::TIMESTAMP_HEADER, timestamp_index);

    for field_spec in FIELDS {
        let index = headers
            .iter()
            .position(|header| header == field_spec.header)
            .ok_or_else(|| anyhow!("missing required '{}' column", field_spec.header))?;
        map.insert(field_spec.header, index);
    }

    Ok(map)
}

fn parse_record(
    record: &csv::StringRecord,
    column_map: &HashMap<&'static str, usize>,
) -> Result<(NaiveDateTime, [f64; FIELD_COUNT])> {
    let timestamp_text = required_cell(record, column_map, parameters::TIMESTAMP_HEADER)?;
    let timestamp = NaiveDateTime::parse_from_str(timestamp_text, parameters::TIMESTAMP_FORMAT)
        .with_context(|| format!("invalid timestamp '{timestamp_text}'"))?;

    let mut values = [0.0; FIELD_COUNT];
    for (field_index, field_spec) in FIELDS.iter().enumerate() {
        let text = required_cell(record, column_map, field_spec.header)?;
        values[field_index] = text
            .parse::<f64>()
            .with_context(|| format!("invalid {} value '{text}'", field_spec.header))?;
    }

    Ok((timestamp, values))
}

fn required_cell<'a>(
    record: &'a csv::StringRecord,
    column_map: &HashMap<&'static str, usize>,
    header: &'static str,
) -> Result<&'a str> {
    let index = *column_map
        .get(header)
        .ok_or_else(|| anyhow!("internal schema map is missing '{header}'"))?;
    record
        .get(index)
        .ok_or_else(|| anyhow!("row is missing '{header}'"))
}

fn is_displayable(field_index: usize, value: f64) -> bool {
    if !value.is_finite() {
        return false;
    }

    match field_index {
        field::T0_BETA
        | field::T0_SH
        | field::T1_BETA
        | field::T1_SH
        | field::T2_BETA
        | field::T2_SH => (value - INVALID_TEMPERATURE_C).abs() > parameters::INVALID_VALUE_EPSILON,
        field::R0 | field::R1 | field::R2 => {
            (value - INVALID_RESISTANCE_OHM).abs() > parameters::INVALID_VALUE_EPSILON
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    const HEADER: &str =
        "timestamp,Vtrack,VccEst,BmpC,BmpPa,BmpMslpPa,V0,R0,T0,T0sh,V1,R1,T1,T1sh,V2,R2,T2,T2sh\n";

    #[test]
    fn elapsed_time_is_relative_to_each_run() {
        let csv = format!(
            "{HEADER}2026-07-22 10:00:00,2.5,5.14,24.0,94000,101000,2.8,10000,24.2,24.1,2.7,4700,24.3,24.2,2.9,10010,24.1,24.0\n\
             2026-07-22 10:00:03,2.5,5.14,24.1,94001,101001,2.8,10001,24.3,24.2,2.7,4701,24.4,24.3,2.9,10011,24.2,24.1\n"
        );

        let report = parse_reader(Cursor::new(csv), PathBuf::from("run.csv")).unwrap();

        assert_eq!(report.run.samples.len(), 2);
        assert_eq!(report.run.samples[0].elapsed_seconds, 0.0);
        assert_eq!(report.run.samples[1].elapsed_seconds, 3.0);
        assert_eq!(report.run.samples[0].values[field::BMP_C], 24.0);
    }

    #[test]
    fn malformed_rows_are_skipped_without_discarding_the_run() {
        let csv = format!(
            "{HEADER}bad timestamp,2.5,5.14,24.0,94000,101000,2.8,10000,24.2,24.1,2.7,4700,24.3,24.2,2.9,10010,24.1,24.0\n\
             2026-07-22 10:00:03,2.5,5.14,24.1,94001,101001,2.8,10001,24.3,24.2,2.7,4701,24.4,24.3,2.9,10011,24.2,24.1\n"
        );

        let report = parse_reader(Cursor::new(csv), PathBuf::from("run.csv")).unwrap();

        assert_eq!(report.skipped_rows, 1);
        assert_eq!(report.run.samples.len(), 1);
    }
}
