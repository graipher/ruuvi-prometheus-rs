use std::time::SystemTime;

use crate::metrics::Metrics;
use ruuvi_decoders::{self, RuuviData};

pub(crate) struct EnvironmentReadings {
    pub temperature: f64,
    pub humidity_ratio: f64,
    pub pressure_hpa: f64,
}

pub(crate) trait HasEnvironment {
    fn environment(&self) -> Option<EnvironmentReadings>;
}

pub(crate) struct MotionReadings {
    pub acceleration_x_g: Option<f64>,
    pub acceleration_y_g: Option<f64>,
    pub acceleration_z_g: Option<f64>,
    pub battery_voltage: Option<f64>,
    pub tx_power: Option<f64>,
    pub movement_count: Option<f64>,
}

pub(crate) trait HasMotion {
    fn motion(&self) -> Option<MotionReadings>;
}

pub(crate) struct AirQualityReadings {
    pub pm1_0: Option<f64>,
    pub pm2_5: Option<f64>,
    pub pm4_0: Option<f64>,
    pub pm10_0: Option<f64>,
    pub co2: Option<f64>,
    pub voc_index: Option<f64>,
    pub nox_index: Option<f64>,
    pub calibrating: Option<f64>,
}

pub(crate) trait HasAirQuality {
    fn air_quality(&self) -> Option<AirQualityReadings>;
}

pub(crate) trait HasSequenceNumber {
    fn sequence_number(&self) -> Option<f64>;
}

pub(crate) fn apply_environment_metrics<T: HasEnvironment>(
    metrics: &Metrics,
    addr: &str,
    data: &T,
) {
    if let Some(env) = data.environment() {
        metrics.set_temperature(addr, env.temperature);
        metrics.set_humidity(addr, env.humidity_ratio);
        if let Some(dew_point) = dew_point_celsius(env.temperature, env.humidity_ratio) {
            metrics.set_dew_point(addr, dew_point);
        }
        metrics.set_pressure(addr, env.pressure_hpa);
    }
}

pub(crate) fn apply_motion_metrics<T: HasMotion>(metrics: &Metrics, addr: &str, data: &T) {
    if let Some(motion) = data.motion() {
        if let Some(acceleration_x) = motion.acceleration_x_g {
            metrics.set_acceleration(addr, "X", acceleration_x);
        }
        if let Some(acceleration_y) = motion.acceleration_y_g {
            metrics.set_acceleration(addr, "Y", acceleration_y);
        }
        if let Some(acceleration_z) = motion.acceleration_z_g {
            metrics.set_acceleration(addr, "Z", acceleration_z);
        }
        if let Some(voltage) = motion.battery_voltage {
            metrics.set_voltage(addr, voltage);
        }
        if let Some(tx_power) = motion.tx_power {
            metrics.set_tx_power(addr, tx_power);
        }
        if let Some(movement_count) = motion.movement_count {
            metrics.set_move_count(addr, movement_count);
        }
    }
}

const AQI_MAX: f64 = 100.;
const PM2_5_MAX: f64 = 60.;
const PM2_5_MIN: f64 = 0.;
const PM2_5_SCALE: f64 = AQI_MAX / (PM2_5_MAX - PM2_5_MIN); // ≈ 1.6667
const CO2_MAX: f64 = 2300.;
const CO2_MIN: f64 = 420.;
const CO2_SCALE: f64 = AQI_MAX / (CO2_MAX - CO2_MIN); // ≈ 0.05319

pub(crate) fn apply_air_quality_metrics<T: HasAirQuality>(metrics: &Metrics, addr: &str, data: &T) {
    if let Some(air) = data.air_quality() {
        if let Some(pm1_0) = air.pm1_0 {
            metrics.set_pm1_0(addr, pm1_0);
        }
        if let Some(pm2_5) = air.pm2_5 {
            metrics.set_pm2_5(addr, pm2_5);
            if let Some(co2) = air.co2 {
                let pm2_5_clamped = pm2_5.clamp(PM2_5_MIN, PM2_5_MAX);
                let co2_clamped = co2.clamp(CO2_MIN, CO2_MAX);

                let pm_term = (pm2_5_clamped - PM2_5_MIN) * PM2_5_SCALE;
                let co2_term = (co2_clamped - CO2_MIN) * CO2_SCALE;

                let raw = AQI_MAX - (pm_term.powi(2) + co2_term.powi(2)).sqrt();

                let aqi = if raw.is_nan() {
                    0.0
                } else {
                    raw.clamp(0., AQI_MAX).round()
                };

                metrics.set_air_quality_index(addr, aqi);
            }
        }
        if let Some(pm4_0) = air.pm4_0 {
            metrics.set_pm4_0(addr, pm4_0);
        }
        if let Some(pm10_0) = air.pm10_0 {
            metrics.set_pm10_0(addr, pm10_0);
        }
        if let Some(co2) = air.co2 {
            metrics.set_co2(addr, co2);
        }
        if let Some(voc_index) = air.voc_index {
            metrics.set_voc(addr, voc_index);
        }
        if let Some(nox_index) = air.nox_index {
            metrics.set_nox(addr, nox_index);
        }
        if let Some(calibrating) = air.calibrating {
            metrics.set_calibrating(addr, calibrating);
        }
    }
}

pub(crate) fn apply_sequence_number<T: HasSequenceNumber>(metrics: &Metrics, addr: &str, data: &T) {
    if let Some(seqno) = data.sequence_number() {
        metrics.set_seqno(addr, seqno);
    }
}

pub(crate) fn handle_manufacturer_data(metrics: &Metrics, addr: &str, value: &[u8]) {
    let hex: String = value.iter().map(|b| format!("{:02x}", b)).collect();
    match ruuvi_decoders::decode(hex.as_str()) {
        Ok(data) => {
            #[cfg(debug_assertions)]
            println!("{:?}", data);

            match data {
                RuuviData::V5(v5) => {
                    metrics.inc_ruuvi_frames(addr, "5");
                    apply_environment_metrics(metrics, addr, &v5);
                    apply_motion_metrics(metrics, addr, &v5);
                    apply_sequence_number(metrics, addr, &v5);
                }
                RuuviData::V6(v6) => {
                    metrics.inc_ruuvi_frames(addr, "6");
                    apply_environment_metrics(metrics, addr, &v6);
                    apply_air_quality_metrics(metrics, addr, &v6);
                    apply_sequence_number(metrics, addr, &v6);
                }
                RuuviData::E1(e1) => {
                    metrics.inc_ruuvi_frames(addr, "E1");
                    apply_environment_metrics(metrics, addr, &e1);
                    apply_air_quality_metrics(metrics, addr, &e1);
                    apply_sequence_number(metrics, addr, &e1);
                }
            }

            let timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as f64;
            metrics.set_last_updated(addr, timestamp);
        }
        Err(err) => println!("Error decoding data: {}", err),
    };
}

const DEW_POINT_B: f64 = 17.368;
const DEW_POINT_C: f64 = 238.88;

fn dew_point_celsius(temperature_c: f64, humidity_percent: f64) -> Option<f64> {
    if !(0.0..=1.0).contains(&humidity_percent) || humidity_percent <= 0.0 {
        return None;
    }

    if (DEW_POINT_C + temperature_c).abs() < f64::EPSILON {
        return None;
    }

    let gamma = dew_point_gamma(temperature_c, humidity_percent);
    if (DEW_POINT_B - gamma).abs() < f64::EPSILON {
        return None;
    }

    Some(DEW_POINT_C * gamma / (DEW_POINT_B - gamma))
}

fn dew_point_gamma(temperature_c: f64, humidity_percent: f64) -> f64 {
    humidity_percent.ln() + (DEW_POINT_B * temperature_c) / (DEW_POINT_C + temperature_c)
}

impl HasEnvironment for ruuvi_decoders::v5::DataFormatV5 {
    fn environment(&self) -> Option<EnvironmentReadings> {
        Some(EnvironmentReadings {
            temperature: self.temperature?,
            humidity_ratio: self.humidity? / 100.0,
            pressure_hpa: self.pressure? / 100.0,
        })
    }
}

impl HasMotion for ruuvi_decoders::v5::DataFormatV5 {
    fn motion(&self) -> Option<MotionReadings> {
        Some(MotionReadings {
            acceleration_x_g: self.acceleration_x.map(|v| f64::from(v) / 1000.0),
            acceleration_y_g: self.acceleration_y.map(|v| f64::from(v) / 1000.0),
            acceleration_z_g: self.acceleration_z.map(|v| f64::from(v) / 1000.0),
            battery_voltage: self.battery_voltage.map(|v| f64::from(v) / 1000.0),
            tx_power: self.tx_power.map(f64::from),
            movement_count: self.movement_counter.map(f64::from),
        })
    }
}

impl HasSequenceNumber for ruuvi_decoders::v5::DataFormatV5 {
    fn sequence_number(&self) -> Option<f64> {
        self.measurement_sequence.map(f64::from)
    }
}

impl HasEnvironment for ruuvi_decoders::v6::DataFormatV6 {
    fn environment(&self) -> Option<EnvironmentReadings> {
        Some(EnvironmentReadings {
            temperature: self.temperature?,
            humidity_ratio: self.humidity? / 100.0,
            pressure_hpa: self.pressure?,
        })
    }
}

impl HasAirQuality for ruuvi_decoders::v6::DataFormatV6 {
    fn air_quality(&self) -> Option<AirQualityReadings> {
        Some(AirQualityReadings {
            pm1_0: None,
            pm2_5: self.pm2_5,
            pm4_0: None,
            pm10_0: None,
            co2: self.co2.map(f64::from),
            voc_index: self.voc_index.map(f64::from),
            nox_index: self.nox_index.map(f64::from),
            calibrating: Some(f64::from(self.flags & 0b0000_0001)),
        })
    }
}

impl HasSequenceNumber for ruuvi_decoders::v6::DataFormatV6 {
    fn sequence_number(&self) -> Option<f64> {
        self.measurement_sequence.map(f64::from)
    }
}

impl HasEnvironment for ruuvi_decoders::e1::DataFormatE1 {
    fn environment(&self) -> Option<EnvironmentReadings> {
        Some(EnvironmentReadings {
            temperature: self.temperature?,
            humidity_ratio: self.humidity? / 100.0,
            pressure_hpa: self.pressure?,
        })
    }
}

impl HasAirQuality for ruuvi_decoders::e1::DataFormatE1 {
    fn air_quality(&self) -> Option<AirQualityReadings> {
        Some(AirQualityReadings {
            pm1_0: self.pm1_0,
            pm2_5: self.pm2_5,
            pm4_0: self.pm4_0,
            pm10_0: self.pm10_0,
            co2: self.co2.map(f64::from),
            voc_index: self.voc_index.map(f64::from),
            nox_index: self.nox_index.map(f64::from),
            calibrating: Some(f64::from(self.flags & 0b0000_0001)),
        })
    }
}

impl HasSequenceNumber for ruuvi_decoders::e1::DataFormatE1 {
    fn sequence_number(&self) -> Option<f64> {
        self.measurement_sequence.map(f64::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::metrics::{clear, counter_value, gauge_value, take_snapshot};

    #[test]
    fn dew_point_is_calculated_for_valid_input() {
        let dew_point = dew_point_celsius(20.0, 0.5).expect("dew point calculated");
        assert!((dew_point - 9.2674).abs() < 0.001);
    }

    #[test]
    fn dew_point_returns_none_for_invalid_humidity() {
        assert_eq!(None, dew_point_celsius(10.0, 0.0));
        assert_eq!(None, dew_point_celsius(10.0, -0.1));
        assert_eq!(None, dew_point_celsius(10.0, 1.2));
        assert_eq!(None, dew_point_celsius(-DEW_POINT_C, 0.5));
    }

    #[test]
    fn v5_environment_and_motion_are_scaled() {
        let payload = ruuvi_decoders::v5::DataFormatV5 {
            mac_address: "cbb8334c884f".into(),
            temperature: Some(24.3),
            humidity: Some(53.49),
            pressure: Some(100_044.0),
            acceleration_x: Some(4),
            acceleration_y: Some(-4),
            acceleration_z: Some(1036),
            battery_voltage: Some(2977),
            tx_power: Some(4),
            movement_counter: Some(66),
            measurement_sequence: Some(205),
        };

        let env = payload.environment().expect("environment");
        assert!((env.temperature - 24.3).abs() < f64::EPSILON);
        assert!((env.humidity_ratio - 0.5349).abs() < 1e-6);
        assert!((env.pressure_hpa - 1000.44).abs() < 1e-6);

        let motion = payload.motion().expect("motion");
        assert!((motion.acceleration_x_g.unwrap() - 0.004).abs() < 1e-6);
        assert!((motion.acceleration_y_g.unwrap() + 0.004).abs() < 1e-6);
        assert!((motion.acceleration_z_g.unwrap() - 1.036).abs() < 1e-6);
        assert!((motion.battery_voltage.unwrap() - 2.977).abs() < 1e-6);
        assert!((motion.tx_power.unwrap() - 4.0).abs() < f64::EPSILON);
        assert!((motion.movement_count.unwrap() - 66.0).abs() < f64::EPSILON);

        assert_eq!(Some(205.0), payload.sequence_number());
    }

    #[test]
    fn manufacturer_data_records_v5_metrics() {
        let _guard = crate::test_utils::metrics::guard();
        clear();
        let metrics = Metrics::register();
        let addr = "aa:bb:cc:dd:ee:ff";
        let payload_hex = "0512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F";
        let payload = hex_literal::hex!("0512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F");

        handle_manufacturer_data(&metrics, addr, &payload);

        let decoded = match ruuvi_decoders::decode(payload_hex).expect("decode V5 frame") {
            RuuviData::V5(data) => data,
            _ => panic!("unexpected format"),
        };

        let env = decoded.environment().expect("environment data");
        let motion = decoded.motion().expect("motion data");
        let dew_point = dew_point_celsius(env.temperature, env.humidity_ratio).unwrap();

        let snapshot = take_snapshot();

        assert_eq!(
            Some(1),
            counter_value(
                &snapshot,
                "ruuvi_frames_total",
                &[("device", addr), ("format", "5")]
            )
        );

        assert!(
            gauge_value(&snapshot, "ruuvi_temperature_celsius", &[("device", addr)])
                .is_some_and(|v| (v - env.temperature).abs() < 1e-6)
        );
        assert!(
            gauge_value(&snapshot, "ruuvi_humidity_ratio", &[("device", addr)])
                .is_some_and(|v| (v - env.humidity_ratio).abs() < 1e-6)
        );
        assert!(
            gauge_value(&snapshot, "ruuvi_dew_point_celsius", &[("device", addr)])
                .is_some_and(|v| (v - dew_point).abs() < 1e-6)
        );
        assert!(
            gauge_value(&snapshot, "ruuvi_pressure_hpa", &[("device", addr)])
                .is_some_and(|v| (v - env.pressure_hpa).abs() < 1e-6)
        );

        assert!(
            gauge_value(
                &snapshot,
                "ruuvi_acceleration_g",
                &[("device", addr), ("axis", "X")]
            )
            .is_some_and(|v| (v - motion.acceleration_x_g.unwrap()).abs() < 1e-6)
        );
        assert!(
            gauge_value(
                &snapshot,
                "ruuvi_acceleration_g",
                &[("device", addr), ("axis", "Y")]
            )
            .is_some_and(|v| (v - motion.acceleration_y_g.unwrap()).abs() < 1e-6)
        );
        assert!(
            gauge_value(
                &snapshot,
                "ruuvi_acceleration_g",
                &[("device", addr), ("axis", "Z")]
            )
            .is_some_and(|v| (v - motion.acceleration_z_g.unwrap()).abs() < 1e-6)
        );
        assert!(
            gauge_value(&snapshot, "ruuvi_battery_volts", &[("device", addr)])
                .is_some_and(|v| (v - motion.battery_voltage.unwrap()).abs() < 1e-6)
        );
        assert!(
            gauge_value(&snapshot, "ruuvi_txpower_dbm", &[("device", addr)])
                .is_some_and(|v| (v - motion.tx_power.unwrap()).abs() < f64::EPSILON)
        );
        assert!(
            gauge_value(&snapshot, "ruuvi_movecount_total", &[("device", addr)])
                .is_some_and(|v| (v - motion.movement_count.unwrap()).abs() < f64::EPSILON)
        );
        assert!(
            gauge_value(&snapshot, "ruuvi_seqno_current", &[("device", addr)])
                .is_some_and(
                    |v| (v - decoded.measurement_sequence.unwrap() as f64).abs() < f64::EPSILON
                )
        );
    }
}
