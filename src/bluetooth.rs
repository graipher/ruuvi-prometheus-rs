use std::collections::HashSet;
use std::sync::Arc;

use bluer::DeviceEvent::{self, PropertyChanged};
use bluer::DeviceProperty::{AdvertisingFlags, ManufacturerData, Rssi};
use bluer::monitor::{
    Monitor, MonitorEvent, MonitorHandle, MonitorManager, Pattern, RssiSamplingPeriod, Type,
    data_type::MANUFACTURER_SPECIFIC_DATA,
};
use bluer::{Adapter, Device, Session};
use futures::{Stream, StreamExt};
use tokio::sync::Mutex;

use crate::metrics::Metrics;
use crate::ruuvi::handle_manufacturer_data;

fn manufacturer_pattern() -> Pattern {
    let data_type: u8 = MANUFACTURER_SPECIFIC_DATA;
    let start_position: u8 = 0x00;
    let content: Vec<u8> = vec![0x99, 0x04];
    Pattern {
        data_type,
        start_position,
        content,
    }
}

async fn init_adapter(session: &Session, preferred: Option<&str>) -> bluer::Result<Adapter> {
    choose_adapter(
        preferred,
        |name| session.adapter(name),
        || session.default_adapter(),
    )
    .await
}

fn format_device_address(address: &bluer::Address) -> String {
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
        process_events_stream(&mut events, metrics, &addr, active_devices.clone()).await;
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
    seed_from_properties_iter(
        dev.all_properties().await.unwrap(),
        metrics,
        addr,
        Some(dev),
    );
}

async fn mark_active(active_devices: &Arc<Mutex<HashSet<String>>>, addr: &str) -> bool {
    let mut active = active_devices.lock().await;
    if !active.insert(addr.to_string()) {
        return false;
    }
    true
}

fn handle_device_property(
    metrics: &Metrics,
    addr: &str,
    event: DeviceEvent,
    _dev: Option<&Device>,
) {
    match event {
        PropertyChanged(ManufacturerData(data)) => match data.get(&0x0499) {
            Some(value) => handle_manufacturer_data(metrics, addr, value),
            None => eprintln!("No data found"),
        },
        PropertyChanged(Rssi(rssi)) => {
            metrics.set_signal_rssi(addr, rssi as f64);
            #[cfg(debug_assertions)]
            if let Some(dev) = _dev {
                println!("{:?} RSSI: {}", dev, rssi);
            }
        }
        PropertyChanged(AdvertisingFlags(_flags)) =>
        {
            #[cfg(debug_assertions)]
            if let Some(dev) = _dev {
                println!("{:?} AdvertisingFlags: {:?}", dev, _flags);
            }
        }
        _ => eprintln!("Unknown event: {:?}", event),
    }
}

async fn choose_adapter<FPreferred, FDefault, FDefaultFuture>(
    preferred: Option<&str>,
    preferred_lookup: FPreferred,
    default_lookup: FDefault,
) -> bluer::Result<Adapter>
where
    FPreferred: Fn(&str) -> bluer::Result<Adapter>,
    FDefault: FnOnce() -> FDefaultFuture,
    FDefaultFuture: std::future::Future<Output = bluer::Result<Adapter>>,
{
    if let Some(name) = preferred
        && let Ok(adapter) = preferred_lookup(name)
    {
        return Ok(adapter);
    }

    default_lookup().await
}

async fn process_events_stream<S>(
    events: &mut S,
    metrics: Metrics,
    addr: &str,
    active_devices: Arc<Mutex<HashSet<String>>>,
) where
    S: Stream<Item = DeviceEvent> + Unpin,
{
    while let Some(ev) = events.next().await {
        handle_device_property(&metrics, addr, ev, None);
    }
    active_devices.lock().await.remove(addr);
}

fn seed_from_properties_iter<I>(properties: I, metrics: &Metrics, addr: &str, dev: Option<&Device>)
where
    I: IntoIterator<Item = bluer::DeviceProperty>,
{
    for property in properties {
        if let ManufacturerData(data) = property {
            handle_device_property(metrics, addr, PropertyChanged(ManufacturerData(data)), dev);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::metrics::{clear, counter_value, gauge_value, take_snapshot};
    use bluer::ErrorKind;
    use futures::stream;
    use std::sync::Arc as StdArc;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn manufacturer_pattern_matches_ruuvi_prefix() {
        let pattern = manufacturer_pattern();

        assert_eq!(MANUFACTURER_SPECIFIC_DATA, pattern.data_type);
        assert_eq!(0, pattern.start_position);
        assert_eq!(vec![0x99, 0x04], pattern.content);
    }

    #[test]
    fn device_addresses_are_formatted_lowercase() {
        let addr = bluer::Address([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        assert_eq!("aa:bb:cc:dd:ee:ff", format_device_address(&addr));
    }

    #[tokio::test]
    async fn mark_active_allows_first_seen_only_once() {
        let active = Arc::new(Mutex::new(HashSet::new()));

        assert!(mark_active(&active, "aa:bb").await);
        assert!(!mark_active(&active, "aa:bb").await);
        assert!(mark_active(&active, "cc:dd").await);
    }

    #[test]
    fn manufacturer_data_is_forwarded() {
        let _guard = crate::test_utils::metrics::guard();
        clear();
        let metrics = Metrics::register();
        let mut map = std::collections::HashMap::new();
        let payload = hex_literal::hex!("0512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F");
        map.insert(0x0499, payload.to_vec());

        handle_device_property(
            &metrics,
            "aa:bb",
            DeviceEvent::PropertyChanged(ManufacturerData(map)),
            None,
        );

        let snapshot = take_snapshot();
        assert_eq!(
            Some(1),
            counter_value(
                &snapshot,
                "ruuvi_frames_total",
                &[("device", "aa:bb"), ("format", "5")]
            )
        );
    }

    #[test]
    fn non_ruuvi_manufacturer_data_is_ignored() {
        let _guard = crate::test_utils::metrics::guard();
        clear();
        let metrics = Metrics::register();
        let mut map = std::collections::HashMap::new();
        map.insert(0x1234, vec![0xde, 0xad, 0xbe, 0xef]);

        handle_device_property(
            &metrics,
            "aa:bb",
            DeviceEvent::PropertyChanged(ManufacturerData(map)),
            None,
        );

        let snapshot = take_snapshot();
        let value = counter_value(
            &snapshot,
            "ruuvi_frames_total",
            &[("device", "aa:bb"), ("format", "5")],
        )
        .unwrap_or(0);
        assert_eq!(0, value);
    }

    #[test]
    fn rssi_updates_metric() {
        let _guard = crate::test_utils::metrics::guard();
        clear();
        let metrics = Metrics::register();

        handle_device_property(
            &metrics,
            "aa:bb",
            DeviceEvent::PropertyChanged(Rssi(-42)),
            None,
        );

        let snapshot = take_snapshot();
        assert!(
            gauge_value(&snapshot, "ruuvi_rssi_dbm", &[("device", "aa:bb")])
                .is_some_and(|v| (v + 42.0).abs() < f64::EPSILON)
        );
    }

    #[test]
    fn advertising_flags_are_ignored() {
        let _guard = crate::test_utils::metrics::guard();
        clear();
        let metrics = Metrics::register();

        handle_device_property(
            &metrics,
            "aa:bb",
            DeviceEvent::PropertyChanged(AdvertisingFlags(vec![0x01, 0x02])),
            None,
        );

        let snapshot = take_snapshot();
        let value = counter_value(
            &snapshot,
            "ruuvi_frames_total",
            &[("device", "aa:bb"), ("format", "5")],
        )
        .unwrap_or(0);
        assert_eq!(0, value);
    }

    #[test]
    fn unrelated_properties_fall_through() {
        let _guard = crate::test_utils::metrics::guard();
        clear();
        let metrics = Metrics::register();

        handle_device_property(
            &metrics,
            "aa:bb",
            DeviceEvent::PropertyChanged(bluer::DeviceProperty::Name("demo".into())),
            None,
        );

        let snapshot = take_snapshot();
        let value = counter_value(
            &snapshot,
            "ruuvi_frames_total",
            &[("device", "aa:bb"), ("format", "5")],
        )
        .unwrap_or(0);
        assert_eq!(0, value);
    }

    #[tokio::test]
    async fn choose_adapter_prefers_requested() {
        let preferred_hit = StdArc::new(AtomicBool::new(false));
        let default_hit = StdArc::new(AtomicBool::new(false));

        let adapter = choose_adapter(
            Some("hci0"),
            {
                let preferred_hit = preferred_hit.clone();
                move |name| {
                    assert_eq!("hci0", name);
                    preferred_hit.store(true, Ordering::SeqCst);
                    Err(bluer::Error {
                        kind: ErrorKind::InvalidArguments,
                        message: String::new(),
                    })
                }
            },
            {
                let default_hit = default_hit.clone();
                move || async move {
                    default_hit.store(true, Ordering::SeqCst);
                    Err(bluer::Error {
                        kind: ErrorKind::NotReady,
                        message: String::new(),
                    })
                }
            },
        )
        .await;

        assert!(preferred_hit.load(Ordering::SeqCst));
        assert!(adapter.is_err());
        assert!(default_hit.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn process_events_stream_records_metrics() {
        let _guard = crate::test_utils::metrics::guard();
        clear();
        let metrics = Metrics::register();
        let active = Arc::new(Mutex::new(HashSet::from(["aa:bb".to_string()])));
        let payload = hex_literal::hex!("0512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F");

        let mut events = stream::iter(vec![
            DeviceEvent::PropertyChanged(Rssi(-20)),
            DeviceEvent::PropertyChanged(ManufacturerData(
                std::iter::once((0x0499, payload.to_vec())).collect(),
            )),
        ]);

        process_events_stream(&mut events, metrics, "aa:bb", active.clone()).await;

        assert!(!active.lock().await.contains("aa:bb"));

        let snapshot = take_snapshot();
        assert!(
            gauge_value(&snapshot, "ruuvi_rssi_dbm", &[("device", "aa:bb")])
                .is_some_and(|v| (v + 20.0).abs() < f64::EPSILON)
        );
        assert_eq!(
            Some(1),
            counter_value(
                &snapshot,
                "ruuvi_frames_total",
                &[("device", "aa:bb"), ("format", "5")]
            )
        );
    }

    #[test]
    fn seed_from_properties_iter_stops_after_first_match() {
        let _guard = crate::test_utils::metrics::guard();
        clear();
        let metrics = Metrics::register();
        let payload = hex_literal::hex!("0512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F");

        seed_from_properties_iter(
            vec![
                ManufacturerData(std::iter::once((0x1234, vec![0x01])).collect()),
                ManufacturerData(std::iter::once((0x0499, payload.to_vec())).collect()),
                ManufacturerData(std::iter::once((0x0499, vec![0x00])).collect()),
            ],
            &metrics,
            "aa:bb",
            None,
        );

        let snapshot = take_snapshot();
        // Only the first manufacturer data (non-Ruuvi) is processed, so no frames counted.
        let value = counter_value(
            &snapshot,
            "ruuvi_frames_total",
            &[("device", "aa:bb"), ("format", "5")],
        )
        .unwrap_or(0);
        assert_eq!(0, value);
    }
}
