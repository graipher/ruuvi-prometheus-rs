use std::collections::HashSet;
use std::sync::Arc;

use bluer::DeviceEvent::PropertyChanged;
use bluer::DeviceProperty::{AdvertisingFlags, ManufacturerData, Rssi};
use bluer::monitor::{
    Monitor, MonitorEvent, MonitorHandle, MonitorManager, Pattern, RssiSamplingPeriod, Type,
    data_type::MANUFACTURER_SPECIFIC_DATA,
};
use bluer::{Adapter, Device, Session};
use futures::StreamExt;
use tokio::sync::Mutex;

use crate::metrics::Metrics;
use crate::ruuvi::handle_manufacturer_data;

pub(crate) fn manufacturer_pattern() -> Pattern {
    let data_type: u8 = MANUFACTURER_SPECIFIC_DATA;
    let start_position: u8 = 0x00;
    let content: Vec<u8> = vec![0x99, 0x04];
    Pattern {
        data_type,
        start_position,
        content,
    }
}

pub(crate) async fn init_adapter(
    session: &Session,
    preferred: Option<&str>,
) -> bluer::Result<Adapter> {
    if let Some(name) = preferred
        && let Ok(adapter) = session.adapter(name)
    {
        return Ok(adapter);
    }
    session.default_adapter().await
}

pub(crate) fn format_device_address(address: &bluer::Address) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        address.0[0], address.0[1], address.0[2], address.0[3], address.0[4], address.0[5]
    )
}

pub(crate) async fn setup_adapter_monitor(
    preferred: Option<&str>,
) -> bluer::Result<(Adapter, MonitorHandle, MonitorManager)> {
    let pattern = manufacturer_pattern();
    let session = bluer::Session::new().await?;
    let adapter = init_adapter(&session, preferred).await?;
    println!(
        "Running le_passive_scan on adapter {} with or-pattern {:?}",
        adapter.name(),
        pattern
    );
    adapter.set_powered(true).await?;
    let monitor_manager = adapter.monitor().await?;
    let monitor_handle = monitor_manager
        .register(Monitor {
            monitor_type: Type::OrPatterns,
            rssi_low_threshold: None,
            rssi_high_threshold: None,
            rssi_low_timeout: None,
            rssi_high_timeout: None,
            rssi_sampling_period: Some(RssiSamplingPeriod::First),
            patterns: Some(vec![pattern]),
            ..Default::default()
        })
        .await?;

    Ok((adapter, monitor_handle, monitor_manager))
}

pub(crate) async fn scan_and_listen(
    adapter: Adapter,
    mut monitor_handle: MonitorHandle,
    metrics: Metrics,
) -> bluer::Result<()> {
    let active_devices = Arc::new(Mutex::new(HashSet::new()));
    while let Some(mevt) = &monitor_handle.next().await {
        if let MonitorEvent::DeviceFound(devid) = mevt {
            #[cfg(debug_assertions)]
            println!("Discovered device {:?}", devid);
            let dev = adapter.device(devid.device)?;
            let addr = format_device_address(&dev.address());
            if let Some(rssi) = dev.rssi().await? {
                metrics.set_signal_rssi(&addr, rssi as f64);
                #[cfg(debug_assertions)]
                println!("{:?} RSSI: {}", dev, rssi);
            }

            if !mark_active(&active_devices, &addr).await {
                continue;
            }

            seed_from_properties(&dev, &metrics, &addr).await;

            let active_devices = active_devices.clone();
            tokio::spawn(async move {
                handle_device_events(dev, metrics, addr, active_devices).await;
            });
        }
    }
    Ok(())
}

async fn handle_device_events(
    dev: Device,
    metrics: Metrics,
    addr: String,
    active_devices: Arc<Mutex<HashSet<String>>>,
) {
    let result: bluer::Result<()> = async {
        let mut events = dev.events().await?;
        while let Some(ev) = events.next().await {
            match ev {
                PropertyChanged(ManufacturerData(data)) => match data.get(&0x0499) {
                    Some(value) => handle_manufacturer_data(&metrics, &addr, value),
                    None => eprintln!("No data found"),
                },
                PropertyChanged(Rssi(rssi)) => {
                    metrics.set_signal_rssi(&addr, rssi as f64);
                    #[cfg(debug_assertions)]
                    println!("{:?} RSSI: {}", dev, rssi);
                }
                PropertyChanged(AdvertisingFlags(_flags)) => {
                    #[cfg(debug_assertions)]
                    println!("{:?} AdvertisingFlags: {:?}", dev, _flags);
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
}

async fn seed_from_properties(dev: &Device, metrics: &Metrics, addr: &str) {
    #[cfg(debug_assertions)]
    println!("All properties: {:?}", dev.all_properties().await.unwrap());
    for property in dev.all_properties().await.unwrap() {
        if let ManufacturerData(data) = property {
            match data.get(&0x0499) {
                Some(value) => handle_manufacturer_data(metrics, addr, value),
                None => eprintln!("No data found"),
            }
            break;
        }
    }
}

async fn mark_active(active_devices: &Arc<Mutex<HashSet<String>>>, addr: &str) -> bool {
    let mut active = active_devices.lock().await;
    if !active.insert(addr.to_string()) {
        return false;
    }
    true
}
