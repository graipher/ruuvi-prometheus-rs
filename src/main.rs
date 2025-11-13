use std::env;
use std::time::SystemTime;

use bluer::DeviceEvent;
use bluer::DeviceProperty::{ManufacturerData, Rssi};
use bluer::monitor::{Monitor, MonitorEvent, Pattern, RssiSamplingPeriod};
use futures::StreamExt;
use prometheus_exporter;
use ruuvi_decoders::{self, RuuviData};

mod metrics;
use crate::metrics::Metrics;

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    let port = env::var("PORT").unwrap_or("9185".to_string());
    let binding = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Listening on {}", binding);
    let _exporter = prometheus_exporter::start(binding).unwrap();

    let adapter_name = Some("hci0".to_string());
    let data_type: u8 = 0xff;
    let start_position: u8 = 0x00;
    let content: Vec<u8> = vec![0x99, 0x04];
    let pattern = Pattern {
        data_type,
        start_position,
        content,
    };
    let session = bluer::Session::new().await?;
    let adapter = match adapter_name {
        Some(name) => session.adapter(&name)?,
        None => session.default_adapter().await?,
    };

    let metrics = Metrics::register();

    println!(
        "Running le_passive_scan on adapter {} with or-pattern {:?}",
        adapter.name(),
        pattern
    );
    adapter.set_powered(true).await?;
    let mm = adapter.monitor().await?;
    let mut monitor_handle = mm
        .register(Monitor {
            monitor_type: bluer::monitor::Type::OrPatterns,
            rssi_low_threshold: None,
            rssi_high_threshold: None,
            rssi_low_timeout: None,
            rssi_high_timeout: None,
            rssi_sampling_period: Some(RssiSamplingPeriod::First),
            patterns: Some(vec![pattern]),
            ..Default::default()
        })
        .await?;

    while let Some(mevt) = &monitor_handle.next().await {
        if let MonitorEvent::DeviceFound(devid) = mevt {
            println!("Discovered device {:?}", devid);
            let dev = adapter.device(devid.device)?;
            let metrics = metrics.clone();
            tokio::spawn(async move {
                let mut events = dev.events().await.unwrap();
                while let Some(ev) = events.next().await {
                    let addr = format_device_address(&dev.address());

                    match ev {
                        DeviceEvent::PropertyChanged(ManufacturerData(data)) => {
                            match data.get(&0x0499) {
                                Some(value) => {
                                    let hex: String =
                                        value.iter().map(|b| format!("{:02x}", b)).collect();
                                    match ruuvi_decoders::decode(hex.as_str()) {
                                        Ok(data) => {
                                            println!("{:?}", data);

                                            match data {
                                                RuuviData::V5(v5) => {
                                                    metrics
                                                        .ruuvi_frames
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .inc();
                                                    let temperature = v5.temperature.unwrap();
                                                    let humidity = v5.humidity.unwrap() / 100.0;
                                                    metrics
                                                        .temperature
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(temperature);
                                                    metrics
                                                        .humidity
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(humidity);
                                                    if let Some(dew_point) =
                                                        dew_point_celsius(temperature, humidity)
                                                    {
                                                        metrics
                                                            .dew_point
                                                            .get_metric_with_label_values(&[&addr])
                                                            .unwrap()
                                                            .set(dew_point);
                                                    }
                                                    metrics
                                                        .pressure
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(v5.pressure.unwrap() / 100.0);
                                                    metrics
                                                        .voltage
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(
                                                            v5.battery_voltage.unwrap() as f64
                                                                / 1000.0,
                                                        );
                                                    metrics
                                                        .acceleration
                                                        .get_metric_with_label_values(&[&addr, "X"])
                                                        .unwrap()
                                                        .set(
                                                            v5.acceleration_x.unwrap() as f64
                                                                / 1000.0,
                                                        );
                                                    metrics
                                                        .acceleration
                                                        .get_metric_with_label_values(&[&addr, "Y"])
                                                        .unwrap()
                                                        .set(
                                                            v5.acceleration_y.unwrap() as f64
                                                                / 1000.0,
                                                        );
                                                    metrics
                                                        .acceleration
                                                        .get_metric_with_label_values(&[&addr, "Z"])
                                                        .unwrap()
                                                        .set(
                                                            v5.acceleration_z.unwrap() as f64
                                                                / 1000.0,
                                                        );
                                                    metrics
                                                        .tx_power
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(v5.tx_power.unwrap() as f64);
                                                    metrics
                                                        .seqno
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(
                                                            v5.measurement_sequence.unwrap() as f64
                                                        );
                                                }
                                                RuuviData::V6(v6) => {
                                                    metrics
                                                        .ruuvi_frames
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .inc();
                                                    let temperature = v6.temperature.unwrap();
                                                    metrics
                                                        .temperature
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(temperature);
                                                    let humidity = v6.humidity.unwrap() / 100.0;
                                                    metrics
                                                        .humidity
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(humidity);
                                                    if let Some(dew_point) =
                                                        dew_point_celsius(temperature, humidity)
                                                    {
                                                        metrics
                                                            .dew_point
                                                            .get_metric_with_label_values(&[&addr])
                                                            .unwrap()
                                                            .set(dew_point);
                                                    }
                                                    metrics
                                                        .pressure
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(v6.pressure.unwrap() / 100.0);
                                                    metrics
                                                        .seqno
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(
                                                            v6.measurement_sequence.unwrap() as f64
                                                        );
                                                    metrics
                                                        .pm2_5
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(v6.pm2_5.unwrap());
                                                    metrics
                                                        .co2
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(v6.co2.unwrap() as f64);
                                                    metrics
                                                        .voc
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(v6.voc_index.unwrap() as f64);
                                                    metrics
                                                        .nox
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(v6.nox_index.unwrap() as f64);
                                                    let calibrating =
                                                        if (v6.flags & 0b0000_0001) != 0 {
                                                            1.0
                                                        } else {
                                                            0.0
                                                        };
                                                    metrics
                                                        .calibrating
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(calibrating);
                                                }
                                                _ => {}
                                            }

                                            let timestamp = SystemTime::now()
                                                .duration_since(SystemTime::UNIX_EPOCH)
                                                .unwrap()
                                                .as_secs()
                                                as f64;
                                            metrics
                                                .last_updated_ruuvi
                                                .get_metric_with_label_values(&[&addr])
                                                .unwrap()
                                                .set(timestamp);
                                        }
                                        Err(err) => println!("Error decoding data: {}", err),
                                    };
                                }
                                None => println!("No value found"),
                            }
                        }
                        DeviceEvent::PropertyChanged(Rssi(rssi)) => {
                            metrics
                                .signal_rssi
                                .get_metric_with_label_values(&[&addr])
                                .unwrap()
                                .set(rssi as f64);
                            println!("{:?} RSSI: {}", dev, rssi);
                        }
                        _ => {
                            println!("Unknown event: {:?}", ev)
                        }
                    }
                }
            });
        }
    }

    Ok(())
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

fn format_device_address(address: &bluer::Address) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        address.0[0], address.0[1], address.0[2], address.0[3], address.0[4], address.0[5]
    )
}
