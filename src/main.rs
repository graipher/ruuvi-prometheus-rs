mod bluetooth;
mod config;
mod metrics;
mod ruuvi;
#[cfg(test)]
mod test_utils;
use crate::bluetooth::{scan_and_listen, setup_adapter_monitor};
use crate::config::Config;
use crate::metrics::{Metrics, install_prometheus, spawn_process_collector};

#[tokio::main]
async fn main() -> bluer::Result<()> {
    let config = Config::from_env();

    install_prometheus(config.binding, config.idle_timeout);
    println!("Listening on {}", config.binding);

    if config.enable_process_collection {
        spawn_process_collector(config.process_collection_interval);
    }
    let metrics = Metrics::register();

    let (adapter, monitor_handle, _monitor_manager) =
        setup_adapter_monitor(Some(config.adapter_name.as_str())).await?;
    scan_and_listen(adapter, monitor_handle, metrics).await?;

    Ok(())
}
