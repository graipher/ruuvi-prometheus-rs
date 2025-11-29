use duration_string::DurationString;
use std::env;
use std::net::SocketAddr;
use std::time::Duration;

mod bluetooth;
mod metrics;
mod ruuvi;
use crate::bluetooth::{scan_and_listen, setup_adapter_monitor};
use crate::metrics::{Metrics, install_prometheus, spawn_process_collector};

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    let (binding, idle_timeout, collection_interval, adapter_name) = config_from_env();
    install_prometheus(binding, idle_timeout);
    println!("Listening on {}", binding);
    spawn_process_collector(collection_interval);
    let metrics = Metrics::register();

    let (adapter, monitor_handle, _monitor_manager) =
        setup_adapter_monitor(Some(adapter_name.as_str())).await?;
    scan_and_listen(adapter, monitor_handle, metrics).await?;

    Ok(())
}

fn config_from_env() -> (SocketAddr, Duration, Duration, String) {
    let port = env::var("PORT").unwrap_or("9185".to_string());
    let binding: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    let idle_timeout: Duration = env::var("IDLE_TIMEOUT")
        .unwrap_or("60s".to_string())
        .parse::<DurationString>()
        .unwrap()
        .into();
    let collection_interval: Duration = env::var("COLLECTION_INTERVAL")
        .unwrap_or("10s".to_string())
        .parse::<DurationString>()
        .unwrap()
        .into();
    let adapter_name = env::var("BLUETOOTH_DEVICE").unwrap_or("hci0".to_string());
    (binding, idle_timeout, collection_interval, adapter_name)
}
