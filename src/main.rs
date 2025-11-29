mod bluetooth;
mod config;
mod metrics;
mod ruuvi;
use crate::bluetooth::{scan_and_listen, setup_adapter_monitor};
use crate::config::config_from_env;
use crate::metrics::{Metrics, install_prometheus, spawn_process_collector};

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    let config = config_from_env();
    install_prometheus(config.binding, config.idle_timeout);
    println!("Listening on {}", config.binding);
    spawn_process_collector(config.process_collection_interval);
    let metrics = Metrics::register();

    let (adapter, monitor_handle, _monitor_manager) =
        setup_adapter_monitor(Some(config.adapter_name.as_str())).await?;
    scan_and_listen(adapter, monitor_handle, metrics).await?;

    Ok(())
}
