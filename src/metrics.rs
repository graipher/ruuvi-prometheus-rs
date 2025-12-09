use std::net::SocketAddr;
use std::time::Duration;

use metrics::{counter, describe_counter, describe_gauge, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_process::Collector as ProcessCollector;
use metrics_util::MetricKindMask;
use tokio::time;

#[derive(Clone, Copy)]
pub struct Metrics;

impl Metrics {
    const LABEL_DEVICE: &'static str = "device";
    const LABEL_AXIS: &'static str = "axis";
    const LABEL_FORMAT: &'static str = "format";

    pub fn register() -> Self {
        Self::describe_metrics();
        record_rust_info();
        Self
    }

    pub fn inc_ruuvi_frames(&self, device: &str, format: &str) {
        let device_label = device.to_owned();
        let format_label = format.to_owned();
        counter!("ruuvi_frames_total", Self::LABEL_DEVICE => device_label, Self::LABEL_FORMAT => format_label).increment(1);
    }

    pub fn set_temperature(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_temperature_celsius", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_humidity(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_humidity_ratio", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_dew_point(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_dew_point_celsius", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_pressure(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_pressure_hpa", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_acceleration(&self, device: &str, axis: &str, value: f64) {
        let device_label = device.to_owned();
        let axis_label = axis.to_owned();
        gauge!(
            "ruuvi_acceleration_g",
            Self::LABEL_DEVICE => device_label,
            Self::LABEL_AXIS => axis_label
        )
        .set(value);
    }

    pub fn set_voltage(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_battery_volts", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_signal_rssi(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_rssi_dbm", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_tx_power(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_txpower_dbm", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_seqno(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_seqno_current", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_pm1_0(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_pm1_0_ug_m3", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_pm2_5(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_pm2_5_ug_m3", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_pm4_0(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_pm4_0_ug_m3", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_pm10_0(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_pm10_0_ug_m3", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_co2(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_co2_ppm", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_voc(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_voc_index", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_nox(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_nox_index", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_calibrating(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_air_calibrating", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_last_updated(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_last_updated", Self::LABEL_DEVICE => device_label).set(value);
    }

    pub fn set_move_count(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_movecount_total", Self::LABEL_DEVICE => device_label).set(value);
    }

    fn describe_metrics() {
        describe_counter!("ruuvi_frames_total", "Total Ruuvi frames received");
        describe_gauge!("ruuvi_temperature_celsius", "Ruuvi tag sensor temperature");
        describe_gauge!("ruuvi_humidity_ratio", "Ruuvi tag sensor relative humidity");
        describe_gauge!(
            "ruuvi_dew_point_celsius",
            "Calculated dew point derived from temperature and humidity"
        );
        describe_gauge!("ruuvi_pressure_hpa", "Ruuvi tag sensor air pressure");
        describe_gauge!(
            "ruuvi_acceleration_g",
            "Ruuvi tag sensor acceleration X/Y/Z"
        );
        describe_gauge!("ruuvi_battery_volts", "Ruuvi tag battery voltage");
        describe_gauge!("ruuvi_rssi_dbm", "Ruuvi tag received signal strength RSSI");
        describe_gauge!("ruuvi_txpower_dbm", "Ruuvi transmit power in dBm");
        describe_gauge!("ruuvi_seqno_current", "Ruuvi frame sequence number");
        describe_gauge!("ruuvi_pm1_0_ug_m3", "Ruuvi PM1.0 concentration in ug/m3");
        describe_gauge!("ruuvi_pm2_5_ug_m3", "Ruuvi PM2.5 concentration in ug/m3");
        describe_gauge!("ruuvi_pm4_0_ug_m3", "Ruuvi PM4.0 concentration in ug/m3");
        describe_gauge!("ruuvi_pm10_0_ug_m3", "Ruuvi PM10.0 concentration in ug/m3");
        describe_gauge!("ruuvi_co2_ppm", "Ruuvi CO2 concentration in ppm");
        describe_gauge!("ruuvi_voc_index", "Ruuvi VOC index");
        describe_gauge!("ruuvi_nox_index", "Ruuvi NOx index");
        describe_gauge!("ruuvi_air_calibrating", "Ruuvi calibrating");
        describe_gauge!("ruuvi_last_updated", "Last update of RuuviTag");
        describe_gauge!("rust_info", "Info about the Rust version");
        describe_gauge!("ruuvi_movecount_total", "Ruuvi movement counter");
    }
}

fn record_rust_info() {
    let compile_datetime = compile_time::datetime_str!();
    let rustc_version = compile_time::rustc_version_str!();
    gauge!(
        "rust_info",
        "rustc_version" => rustc_version,
        "compile_time" => compile_datetime,
        "version" => env!("CARGO_PKG_VERSION")
    )
    .set(1.0);
}

pub(crate) fn install_prometheus(binding: SocketAddr, timeout: Duration) {
    PrometheusBuilder::new()
        .with_http_listener(binding)
        .idle_timeout(MetricKindMask::ALL, Some(timeout))
        .install()
        .expect("failed to install Prometheus exporter");
}

pub(crate) fn spawn_process_collector(collection_interval: Duration) {
    let process_collector = ProcessCollector::default();
    process_collector.describe();
    process_collector.collect();
    tokio::spawn(async move {
        let mut interval = time::interval(collection_interval);
        loop {
            interval.tick().await;
            process_collector.collect();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::metrics::{clear, counter_value, gauge_value, take_snapshot};

    #[test]
    fn register_records_rust_info() {
        let _guard = crate::test_utils::metrics::guard();
        clear();

        let _metrics = Metrics::register();
        let snapshot = take_snapshot();

        assert!(
            gauge_value(
                &snapshot,
                "rust_info",
                &[
                    ("rustc_version", compile_time::rustc_version_str!()),
                    ("compile_time", compile_time::datetime_str!()),
                    ("version", env!("CARGO_PKG_VERSION"))
                ]
            )
            .is_some_and(|v| (v - 1.0).abs() < f64::EPSILON)
        );
    }

    #[test]
    fn counters_and_gauges_are_labeled() {
        let _guard = crate::test_utils::metrics::guard();
        clear();
        let metrics = Metrics::register();

        metrics.inc_ruuvi_frames("aa:bb", "5");
        metrics.set_signal_rssi("aa:bb", -55.0);
        metrics.set_acceleration("aa:bb", "Z", 0.123);

        let snapshot = take_snapshot();

        assert_eq!(
            Some(1),
            counter_value(
                &snapshot,
                "ruuvi_frames_total",
                &[("device", "aa:bb"), ("format", "5")]
            )
        );
        assert!(
            gauge_value(&snapshot, "ruuvi_rssi_dbm", &[("device", "aa:bb")])
                .is_some_and(|v| (v + 55.0).abs() < f64::EPSILON)
        );
        assert!(
            gauge_value(
                &snapshot,
                "ruuvi_acceleration_g",
                &[("device", "aa:bb"), ("axis", "Z")]
            )
            .is_some_and(|v| (v - 0.123).abs() < f64::EPSILON)
        );
    }

    #[test]
    fn air_quality_and_misc_metrics_are_recorded() {
        let _guard = crate::test_utils::metrics::guard();
        clear();
        let metrics = Metrics::register();

        metrics.set_pm1_0("aa:bb", 1.1);
        metrics.set_pm2_5("aa:bb", 2.2);
        metrics.set_pm4_0("aa:bb", 4.4);
        metrics.set_pm10_0("aa:bb", 10.1);
        metrics.set_co2("aa:bb", 400.0);
        metrics.set_voc("aa:bb", 50.0);
        metrics.set_nox("aa:bb", 25.0);
        metrics.set_calibrating("aa:bb", 1.0);
        metrics.set_last_updated("aa:bb", 123.0);
        metrics.set_move_count("aa:bb", 7.0);
        metrics.set_voltage("aa:bb", 2.9);
        metrics.set_tx_power("aa:bb", -4.0);
        metrics.set_seqno("aa:bb", 42.0);

        let snapshot = take_snapshot();

        let expect = |name: &str, value: f64| {
            assert!(
                gauge_value(&snapshot, name, &[("device", "aa:bb")])
                    .is_some_and(|v| (v - value).abs() < f64::EPSILON)
            );
        };

        expect("ruuvi_pm1_0_ug_m3", 1.1);
        expect("ruuvi_pm2_5_ug_m3", 2.2);
        expect("ruuvi_pm4_0_ug_m3", 4.4);
        expect("ruuvi_pm10_0_ug_m3", 10.1);
        expect("ruuvi_co2_ppm", 400.0);
        expect("ruuvi_voc_index", 50.0);
        expect("ruuvi_nox_index", 25.0);
        expect("ruuvi_air_calibrating", 1.0);
        expect("ruuvi_last_updated", 123.0);
        expect("ruuvi_movecount_total", 7.0);
        expect("ruuvi_battery_volts", 2.9);
        expect("ruuvi_txpower_dbm", -4.0);
        expect("ruuvi_seqno_current", 42.0);
    }
}
