use std::env;
use std::sync::Arc;
use std::time::SystemTime;

use bluer::DeviceEvent;
use bluer::DeviceProperty::{ManufacturerData, Rssi};
use bluer::monitor::{Monitor, MonitorEvent, Pattern, RssiSamplingPeriod};
use futures::StreamExt;
use prometheus_exporter::prometheus::register_counter_vec;
use prometheus_exporter::{self, prometheus::register_gauge, prometheus::register_gauge_vec};
use ruuvi_decoders::{self, RuuviData};

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

    let ruuvi_frames = Arc::new(
        register_counter_vec!(
            "ruuvi_frames_total",
            "Total Ruuvi frames received",
            &["device"]
        )
        .unwrap(),
    );
    let temperature = Arc::new(
        register_gauge_vec!(
            "ruuvi_temperature_celsius",
            "Ruuvi tag sensor temperature",
            &["device"]
        )
        .unwrap(),
    );
    let humidity = Arc::new(
        register_gauge_vec!(
            "ruuvi_humidity_ratio",
            "Ruuvi tag sensor relative humidity",
            &["device"]
        )
        .unwrap(),
    );
    let pressure = Arc::new(
        register_gauge_vec!(
            "ruuvi_pressure_hpa",
            "Ruuvi tag sensor air pressure",
            &["device"]
        )
        .unwrap(),
    );
    let acceleration = Arc::new(
        register_gauge_vec!(
            "ruuvi_acceleration_g",
            "Ruuvi tag sensor acceleration X/Y/Z",
            &["device", "axis"]
        )
        .unwrap(),
    );
    let voltage = Arc::new(
        register_gauge_vec!(
            "ruuvi_battery_volts",
            "Ruuvi tag battery voltage",
            &["device"]
        )
        .unwrap(),
    );
    let signal_rssi = Arc::new(
        register_gauge_vec!(
            "ruuvi_rssi_dbm",
            "Ruuvi tag received signal strength RSSI",
            &["device"]
        )
        .unwrap(),
    );
    let tx_power = Arc::new(
        register_gauge_vec!(
            "ruuvi_txpower_dbm",
            "Ruuvi transmit power in dBm",
            &["device"]
        )
        .unwrap(),
    );
    let last_updated_ruuvi = Arc::new(
        register_gauge_vec!("ruuvi_last_updated", "Last update of RuuviTag", &["mac"]).unwrap(),
    );
    let process_start_time =
        register_gauge!("process_start_time_seconds", "Start time of the process").unwrap();
    process_start_time.set(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as f64,
    );

    let compile_datetime = compile_time::datetime_str!();
    let rustc_version = compile_time::rustc_version_str!();
    let rust_info = register_gauge_vec!(
        "rust_info",
        "Info about the Rust version",
        &["rustc_version", "compile_time", "version"]
    )
    .unwrap();
    rust_info
        .get_metric_with_label_values(&[rustc_version, compile_datetime, env!("CARGO_PKG_VERSION")])
        .unwrap()
        .set(1.);

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
            let ruuvi_frames = Arc::clone(&ruuvi_frames);
            let last_updated_ruuvi = Arc::clone(&last_updated_ruuvi);
            let temperature = Arc::clone(&temperature);
            let humidity = Arc::clone(&humidity);
            let pressure = Arc::clone(&pressure);
            let voltage = Arc::clone(&voltage);
            let acceleration = Arc::clone(&acceleration);
            let signal_rssi = Arc::clone(&signal_rssi);
            let tx_power = Arc::clone(&tx_power);
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
                                                    ruuvi_frames
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .inc();
                                                    temperature
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(v5.temperature.unwrap());
                                                    humidity
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(v5.humidity.unwrap());
                                                    pressure
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(v5.pressure.unwrap());
                                                    voltage
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(v5.battery_voltage.unwrap() as f64);
                                                    acceleration
                                                        .get_metric_with_label_values(&[&addr, "X"])
                                                        .unwrap()
                                                        .set(v5.acceleration_x.unwrap() as f64);
                                                    acceleration
                                                        .get_metric_with_label_values(&[&addr, "Y"])
                                                        .unwrap()
                                                        .set(v5.acceleration_y.unwrap() as f64);
                                                    acceleration
                                                        .get_metric_with_label_values(&[&addr, "Z"])
                                                        .unwrap()
                                                        .set(v5.acceleration_z.unwrap() as f64);
                                                    tx_power
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(v5.tx_power.unwrap() as f64);
                                                }
                                                RuuviData::V6(v6) => {
                                                    ruuvi_frames
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .inc();
                                                    temperature
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(v6.temperature.unwrap());
                                                    humidity
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(v6.humidity.unwrap());
                                                    pressure
                                                        .get_metric_with_label_values(&[&addr])
                                                        .unwrap()
                                                        .set(v6.pressure.unwrap());
                                                }
                                                _ => {}
                                            }

                                            let timestamp = SystemTime::now()
                                                .duration_since(SystemTime::UNIX_EPOCH)
                                                .unwrap()
                                                .as_secs()
                                                as f64;
                                            last_updated_ruuvi
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
                            signal_rssi
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

fn format_device_address(address: &bluer::Address) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        address.0[0], address.0[1], address.0[2], address.0[3], address.0[4], address.0[5]
    )
}
