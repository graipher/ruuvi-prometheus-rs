use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use prometheus_exporter::prometheus::{
    CounterVec, GaugeVec, register_counter_vec, register_gauge, register_gauge_vec,
};

#[derive(Clone)]
pub struct Metrics {
    pub ruuvi_frames: Arc<CounterVec>,
    pub temperature: Arc<GaugeVec>,
    pub humidity: Arc<GaugeVec>,
    pub pressure: Arc<GaugeVec>,
    pub acceleration: Arc<GaugeVec>,
    pub voltage: Arc<GaugeVec>,
    pub signal_rssi: Arc<GaugeVec>,
    pub tx_power: Arc<GaugeVec>,
    pub seqno: Arc<GaugeVec>,
    pub pm2_5: Arc<GaugeVec>,
    pub co2: Arc<GaugeVec>,
    pub voc: Arc<GaugeVec>,
    pub nox: Arc<GaugeVec>,
    pub calibrating: Arc<GaugeVec>,
    pub last_updated_ruuvi: Arc<GaugeVec>,
}

impl Metrics {
    pub fn register() -> Self {
        record_process_start_time();
        record_rust_info();

        Self {
            ruuvi_frames: counter_vec(
                "ruuvi_frames_total",
                "Total Ruuvi frames received",
                &["device"],
            ),
            temperature: gauge_vec(
                "ruuvi_temperature_celsius",
                "Ruuvi tag sensor temperature",
                &["device"],
            ),
            humidity: gauge_vec(
                "ruuvi_humidity_ratio",
                "Ruuvi tag sensor relative humidity",
                &["device"],
            ),
            pressure: gauge_vec(
                "ruuvi_pressure_hpa",
                "Ruuvi tag sensor air pressure",
                &["device"],
            ),
            acceleration: gauge_vec(
                "ruuvi_acceleration_g",
                "Ruuvi tag sensor acceleration X/Y/Z",
                &["device", "axis"],
            ),
            voltage: gauge_vec(
                "ruuvi_battery_volts",
                "Ruuvi tag battery voltage",
                &["device"],
            ),
            signal_rssi: gauge_vec(
                "ruuvi_rssi_dbm",
                "Ruuvi tag received signal strength RSSI",
                &["device"],
            ),
            tx_power: gauge_vec(
                "ruuvi_txpower_dbm",
                "Ruuvi transmit power in dBm",
                &["device"],
            ),
            seqno: gauge_vec(
                "ruuvi_seqno_current",
                "Ruuvi frame sequence number",
                &["device"],
            ),
            pm2_5: gauge_vec(
                "ruuvi_pm2_5_ug_m3",
                "Ruuvi PM2.5 concentration in ug/m3",
                &["device"],
            ),
            co2: gauge_vec(
                "ruuvi_co2_ppm",
                "Ruuvi CO2 concentration in ppm",
                &["device"],
            ),
            voc: gauge_vec("ruuvi_voc_index", "Ruuvi VOC index", &["device"]),
            nox: gauge_vec("ruuvi_nox_index", "Ruuvi NOx index", &["device"]),
            calibrating: gauge_vec("ruuvi_air_calibrating", "Ruuvi calibrating", &["device"]),
            last_updated_ruuvi: gauge_vec(
                "ruuvi_last_updated",
                "Last update of RuuviTag",
                &["device"],
            ),
        }
    }
}

fn counter_vec(name: &str, help: &str, labels: &[&str]) -> Arc<CounterVec> {
    Arc::new(
        register_counter_vec!(name, help, labels)
            .unwrap_or_else(|err| panic!("failed to register counter {}: {}", name, err)),
    )
}

fn gauge_vec(name: &str, help: &str, labels: &[&str]) -> Arc<GaugeVec> {
    Arc::new(
        register_gauge_vec!(name, help, labels)
            .unwrap_or_else(|err| panic!("failed to register gauge {}: {}", name, err)),
    )
}

fn record_process_start_time() {
    let process_start_time =
        register_gauge!("process_start_time_seconds", "Start time of the process")
            .unwrap_or_else(|err| panic!("failed to register process start time gauge: {}", err));
    process_start_time.set(unix_timestamp());
}

fn record_rust_info() {
    let compile_datetime = compile_time::datetime_str!();
    let rustc_version = compile_time::rustc_version_str!();
    let rust_info = register_gauge_vec!(
        "rust_info",
        "Info about the Rust version",
        &["rustc_version", "compile_time", "version"]
    )
    .unwrap_or_else(|err| panic!("failed to register rust_info gauge: {}", err));
    rust_info
        .get_metric_with_label_values(&[rustc_version, compile_datetime, env!("CARGO_PKG_VERSION")])
        .unwrap()
        .set(1.);
}

fn unix_timestamp() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as f64
}
