# xSAR Sensor Logger Display

Version 2 is an oscilloscope-style desktop viewer for CSV logs produced by the
xSAR NTC batch thermal sensor harvester.

## Version 2 behaviour

- Loads the harvester's 18-column CSV schema.
- Loads multiple captures and aligns each one at **elapsed time 0 seconds**.
- Keeps original timestamps as provenance; different calendar dates are not
  treated as one continuous timeline.
- Treats **BMP180 temperature as the reference/ground-truth trace**.
- Treats ADS1115/NTC β and Steinhart–Hart temperatures as provisional
  divider-health and comparative-trend traces.
- Groups temperature, residuals, voltage, resistance, and pressure into stacked
  oscilloscope-style plots.
- Omits the harvester's invalid NTC sentinel values from plots.

## Expected CSV

The authoritative header is:

```text
timestamp,Vtrack,VccEst,BmpC,BmpPa,BmpMslpPa,V0,R0,T0,T0sh,V1,R1,T1,T1sh,V2,R2,T2,T2sh
```

The default file is:

```text
/tmp/RPi_thermal_sensor_log.csv
```

## Build and run

```bash
cargo test
cargo run --release
```

The application is intended for Linux desktop use, including KDE Plasma on
Wayland.
