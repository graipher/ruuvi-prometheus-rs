use std::env;
use std::net::SocketAddr;
use std::time::Duration;

use duration_string::DurationString;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Config {
    pub binding: SocketAddr,
    pub idle_timeout: Duration,
    pub enable_process_collection: bool,
    pub process_collection_interval: Duration,
    pub adapter_name: String,
}

impl Config {
    pub fn from_env() -> Self {
        let binding: SocketAddr = env::var("BINDING")
            .unwrap_or("0.0.0.0:9185".to_string())
            .parse()
            .unwrap();
        let idle_timeout: Duration = env::var("IDLE_TIMEOUT")
            .unwrap_or("60s".to_string())
            .parse::<DurationString>()
            .unwrap()
            .into();
        let enable_process_collection = env::var("ENABLE_PROCESS_COLLECTION")
            .unwrap_or("false".to_string())
            .parse::<bool>()
            .unwrap();
        let process_collection_interval: Duration = env::var("PROCESS_COLLECTION_INTERVAL")
            .unwrap_or("10s".to_string())
            .parse::<DurationString>()
            .unwrap()
            .into();
        let adapter_name = env::var("ADAPTER_NAME").unwrap_or("hci0".to_string());
        Self {
            binding,
            idle_timeout,
            enable_process_collection,
            process_collection_interval,
            adapter_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env(vars: &[(&str, Option<&str>)], f: impl FnOnce()) {
        let _guard = ENV_LOCK.lock().unwrap();
        let previous: Vec<(String, Option<String>)> = vars
            .iter()
            .map(|(key, _)| (key.to_string(), env::var(key).ok()))
            .collect();

        for (key, value) in vars {
            match value {
                Some(v) => unsafe { env::set_var(key, v) },
                None => unsafe { env::remove_var(key) },
            }
        }

        f();

        for (key, value) in previous {
            match value {
                Some(v) => unsafe { env::set_var(&key, v) },
                None => unsafe { env::remove_var(&key) },
            }
        }
    }

    #[test]
    fn loads_defaults_when_env_missing() {
        with_env(
            &[
                ("BINDING", None),
                ("IDLE_TIMEOUT", None),
                ("ENABLE_PROCESS_COLLECTION", None),
                ("PROCESS_COLLECTION_INTERVAL", None),
                ("ADAPTER_NAME", None),
            ],
            || {
                let config = Config::from_env();

                assert_eq!(
                    "0.0.0.0:9185".parse::<SocketAddr>().unwrap(),
                    config.binding
                );
                assert_eq!(Duration::from_secs(60), config.idle_timeout);
                assert!(!config.enable_process_collection);
                assert_eq!(Duration::from_secs(10), config.process_collection_interval);
                assert_eq!("hci0", config.adapter_name);
            },
        );
    }

    #[test]
    fn parses_overrides_from_env() {
        with_env(
            &[
                ("BINDING", Some("127.0.0.1:9999")),
                ("IDLE_TIMEOUT", Some("120s")),
                ("ENABLE_PROCESS_COLLECTION", Some("true")),
                ("PROCESS_COLLECTION_INTERVAL", Some("30s")),
                ("ADAPTER_NAME", Some("usb0")),
            ],
            || {
                let config = Config::from_env();

                assert_eq!(
                    "127.0.0.1:9999".parse::<SocketAddr>().unwrap(),
                    config.binding
                );
                assert_eq!(Duration::from_secs(120), config.idle_timeout);
                assert!(config.enable_process_collection);
                assert_eq!(Duration::from_secs(30), config.process_collection_interval);
                assert_eq!("usb0", config.adapter_name);
            },
        );
    }
}
