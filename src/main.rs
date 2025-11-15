use std::collections::HashSet;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;

use bluer::DeviceEvent;
use bluer::DeviceProperty::{ManufacturerData, Rssi};
use bluer::monitor::{
    Monitor, MonitorEvent, Pattern, RssiSamplingPeriod, data_type::MANUFACTURER_SPECIFIC_DATA,
};
use futures::StreamExt;
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_process::Collector as ProcessCollector;
use ruuvi_decoders::{self, RuuviData};
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

mod metrics;
use crate::metrics::Metrics;

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    let port = env::var("PORT").unwrap_or("9185".to_string());
    let binding: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Listening on {}", binding);
    PrometheusBuilder::new()
        .with_http_listener(binding)
        .install()
        .expect("failed to install Prometheus exporter");

    let process_collector = ProcessCollector::default();
    process_collector.describe();
    process_collector.collect();
    let collector = process_collector.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            collector.collect();
        }
    });

    let data_type: u8 = MANUFACTURER_SPECIFIC_DATA;
    let start_position: u8 = 0x00;
    let content: Vec<u8> = vec![0x99, 0x04];
    let pattern = Pattern {
        data_type,
        start_position,
        content,
    };
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;

    let metrics = Metrics::register();
    let active_devices = Arc::new(Mutex::new(HashSet::new()));

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
        match mevt {
            MonitorEvent::DeviceFound(devid) => {
                #[cfg(debug_assertions)]
                println!("Discovered device {:?}", devid);
                let dev = adapter.device(devid.device)?;
                let addr = format_device_address(&dev.address());
                if let Some(rssi) = dev.rssi().await? {
                    metrics.set_signal_rssi(&addr, rssi as f64);
                    #[cfg(debug_assertions)]
                    println!("{:?} RSSI: {}", dev, rssi);
                }
                #[cfg(debug_assertions)]
                println!("All properties: {:?}", dev.all_properties().await.unwrap());

                for property in dev.all_properties().await.unwrap() {
                    if let ManufacturerData(data) = property {
                        match data.get(&0x0499) {
                            Some(value) => handle_manufacturer_data(&metrics, &addr, value),
                            None => eprintln!("No data found"),
                        }
                    }
                }
                let mut active = active_devices.lock().await;
                if !active.insert(addr.clone()) {
                    continue;
                }
                drop(active);

                let metrics = metrics.clone();
                let active_devices = active_devices.clone();
                tokio::spawn(async move {
                    let result: bluer::Result<()> = async {
                        let mut events = dev.events().await?;
                        while let Some(ev) = events.next().await {
                            match ev {
                                DeviceEvent::PropertyChanged(ManufacturerData(data)) => {
                                    match data.get(&0x0499) {
                                        Some(value) => {
                                            handle_manufacturer_data(&metrics, &addr, value)
                                        }
                                        None => eprintln!("No data found"),
                                    }
                                }
                                DeviceEvent::PropertyChanged(Rssi(rssi)) => {
                                    metrics.set_signal_rssi(&addr, rssi as f64);
                                    #[cfg(debug_assertions)]
                                    println!("{:?} RSSI: {}", dev, rssi);
                                }
                                _ => eprintln!("Unknown event: {:?}", ev),
                            }
                        }
                        Ok(())
                    }
                    .await;

                    if let Err(err) = result {
                        eprintln!("Error processing device {}: {}", addr, err);
                    }

                    active_devices.lock().await.remove(&addr);
                });
            }
            MonitorEvent::DeviceLost(devid) => {
                let dev = adapter.device(devid.device)?;
                let addr = format_device_address(&dev.address());
                #[cfg(debug_assertions)]
                println!("Lost device {:?}", devid);
                active_devices.lock().await.remove(&addr);
            }
            _ => {}
        }
    }

    Ok(())
}

fn handle_manufacturer_data(metrics: &Metrics, addr: &str, value: &[u8]) {
    let hex: String = value.iter().map(|b| format!("{:02x}", b)).collect();
    match ruuvi_decoders::decode(hex.as_str()) {
        Ok(data) => {
            #[cfg(debug_assertions)]
            println!("{:?}", data);

            match data {
                RuuviData::V5(v5) => {
                    metrics.inc_ruuvi_frames(addr);
                    let temperature = v5.temperature.unwrap();
                    let humidity = v5.humidity.unwrap() / 100.0;
                    metrics.set_temperature(addr, temperature);
                    metrics.set_humidity(addr, humidity);
                    if let Some(dew_point) = dew_point_celsius(temperature, humidity) {
                        metrics.set_dew_point(addr, dew_point);
                    }
                    metrics.set_pressure(addr, v5.pressure.unwrap() / 100.0);
                    metrics.set_voltage(addr, v5.battery_voltage.unwrap() as f64 / 1000.0);
                    metrics.set_acceleration(addr, "X", v5.acceleration_x.unwrap() as f64 / 1000.0);
                    metrics.set_acceleration(addr, "Y", v5.acceleration_y.unwrap() as f64 / 1000.0);
                    metrics.set_acceleration(addr, "Z", v5.acceleration_z.unwrap() as f64 / 1000.0);
                    metrics.set_tx_power(addr, v5.tx_power.unwrap() as f64);
                    metrics.set_seqno(addr, v5.measurement_sequence.unwrap() as f64);
                    metrics.set_move_count(addr, v5.movement_counter.unwrap() as f64);
                    metrics.set_format(addr, 5 as f64);
                }
                RuuviData::V6(v6) => {
                    metrics.inc_ruuvi_frames(addr);
                    let temperature = v6.temperature.unwrap();
                    let humidity = v6.humidity.unwrap() / 100.0;
                    metrics.set_temperature(addr, temperature);
                    metrics.set_humidity(addr, humidity);
                    if let Some(dew_point) = dew_point_celsius(temperature, humidity) {
                        metrics.set_dew_point(addr, dew_point);
                    }
                    metrics.set_pressure(addr, v6.pressure.unwrap());
                    metrics.set_seqno(addr, v6.measurement_sequence.unwrap() as f64);
                    metrics.set_pm2_5(addr, v6.pm2_5.unwrap());
                    metrics.set_co2(addr, v6.co2.unwrap() as f64);
                    metrics.set_voc(addr, v6.voc_index.unwrap() as f64);
                    metrics.set_nox(addr, v6.nox_index.unwrap() as f64);
                    let calibrating = if (v6.flags & 0b0000_0001) != 0 {
                        1.0
                    } else {
                        0.0
                    };
                    metrics.set_calibrating(addr, calibrating);
                    metrics.set_format(addr, 6 as f64);
                }
                RuuviData::E1(e1) => {
                    metrics.inc_ruuvi_frames(addr);
                    let temperature = e1.temperature.unwrap();
                    let humidity = e1.humidity.unwrap() / 100.0;
                    metrics.set_temperature(addr, temperature);
                    metrics.set_humidity(addr, humidity);
                    if let Some(dew_point) = dew_point_celsius(temperature, humidity) {
                        metrics.set_dew_point(addr, dew_point);
                    }
                    metrics.set_pressure(addr, e1.pressure.unwrap());
                    metrics.set_seqno(addr, e1.measurement_sequence.unwrap() as f64);
                    metrics.set_pm1_0(addr, e1.pm1_0.unwrap());
                    metrics.set_pm2_5(addr, e1.pm2_5.unwrap());
                    metrics.set_pm4_0(addr, e1.pm4_0.unwrap());
                    metrics.set_pm10_0(addr, e1.pm10_0.unwrap());
                    metrics.set_co2(addr, e1.co2.unwrap() as f64);
                    metrics.set_voc(addr, e1.voc_index.unwrap() as f64);
                    metrics.set_nox(addr, e1.nox_index.unwrap() as f64);
                    let calibrating = if (e1.flags & 0b0000_0001) != 0 {
                        1.0
                    } else {
                        0.0
                    };
                    metrics.set_calibrating(addr, calibrating);
                    metrics.set_format(addr, 225 as f64);
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

fn format_device_address(address: &bluer::Address) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        address.0[0], address.0[1], address.0[2], address.0[3], address.0[4], address.0[5]
    )
}
