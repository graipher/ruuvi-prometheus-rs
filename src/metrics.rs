use metrics::{counter, describe_counter, describe_gauge, gauge};

#[derive(Clone, Copy)]
pub struct Metrics;

impl Metrics {
    const LABEL_DEVICE: &'static str = "device";
    const LABEL_AXIS: &'static str = "axis";

    pub fn register() -> Self {
        Self::describe_metrics();
        record_rust_info();
        Self
    }

    pub fn inc_ruuvi_frames(&self, device: &str) {
        let device_label = device.to_owned();
        counter!("ruuvi_frames_total", Self::LABEL_DEVICE => device_label).increment(1);
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

    pub fn set_pm2_5(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_pm2_5_ug_m3", Self::LABEL_DEVICE => device_label).set(value);
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

    pub fn set_format(&self, device: &str, value: f64) {
        let device_label = device.to_owned();
        gauge!("ruuvi_format", Self::LABEL_DEVICE => device_label).set(value);
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
        describe_gauge!("ruuvi_pm2_5_ug_m3", "Ruuvi PM2.5 concentration in ug/m3");
        describe_gauge!("ruuvi_co2_ppm", "Ruuvi CO2 concentration in ppm");
        describe_gauge!("ruuvi_voc_index", "Ruuvi VOC index");
        describe_gauge!("ruuvi_nox_index", "Ruuvi NOx index");
        describe_gauge!("ruuvi_air_calibrating", "Ruuvi calibrating");
        describe_gauge!("ruuvi_last_updated", "Last update of RuuviTag");
        describe_gauge!("rust_info", "Info about the Rust version");
        describe_gauge!("ruuvi_movecount_total", "Ruuvi movement counter");
        describe_gauge!(
            "ruuvi_format",
            "Ruuvi frame format version (e.g. 3, 5 or 6)"
        );
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
