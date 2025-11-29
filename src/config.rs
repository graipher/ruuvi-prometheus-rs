use std::env;
use std::net::SocketAddr;
use std::time::Duration;

use duration_string::DurationString;

pub struct Config {
    pub binding: SocketAddr,
    pub idle_timeout: Duration,
    pub process_collection_interval: Duration,
    pub adapter_name: String,
}

pub fn config_from_env() -> Config {
    let port = env::var("PORT").unwrap_or("9185".to_string());
    let binding: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    let idle_timeout: Duration = env::var("IDLE_TIMEOUT")
        .unwrap_or("60s".to_string())
        .parse::<DurationString>()
        .unwrap()
        .into();
    let process_collection_interval: Duration = env::var("PROCESS_COLLECTION_INTERVAL")
        .unwrap_or("10s".to_string())
        .parse::<DurationString>()
        .unwrap()
        .into();
    let adapter_name = env::var("BLUETOOTH_DEVICE").unwrap_or("hci0".to_string());

    Config {
        binding,
        idle_timeout,
        process_collection_interval,
        adapter_name,
    }
}
