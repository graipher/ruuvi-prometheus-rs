#![cfg(test)]

pub mod metrics {
    use metrics_util::debugging::{DebugValue, DebuggingRecorder, Snapshotter};
    use std::collections::BTreeMap;
    use std::sync::{Mutex, OnceLock};

    static METRIC_LOCK: Mutex<()> = Mutex::new(());
    static SNAPSHOTTER: OnceLock<Snapshotter> = OnceLock::new();

    fn snapshotter() -> &'static Snapshotter {
        SNAPSHOTTER.get_or_init(|| {
            let recorder = DebuggingRecorder::new();
            let snapshotter = recorder.snapshotter();
            let _ = recorder.install();
            snapshotter
        })
    }

    /// Serialize access to the debugging recorder to avoid cross-test races.
    pub fn guard() -> std::sync::MutexGuard<'static, ()> {
        METRIC_LOCK.lock().expect("metric lock poisoned")
    }

    /// Clears metrics by taking a snapshot and dropping the values.
    pub fn clear() {
        let _ = take_snapshot();
    }

    pub fn take_snapshot() -> Vec<(String, BTreeMap<String, String>, DebugValue)> {
        snapshotter()
            .snapshot()
            .into_vec()
            .into_iter()
            .map(|(key, _, _, value)| {
                let name = key.key().name().to_string();
                let labels = key
                    .key()
                    .labels()
                    .map(|l| (l.key().to_string(), l.value().to_string()))
                    .collect::<BTreeMap<_, _>>();
                (name, labels, value)
            })
            .collect()
    }

    pub fn gauge_value(
        data: &[(String, BTreeMap<String, String>, DebugValue)],
        name: &str,
        labels: &[(&str, &str)],
    ) -> Option<f64> {
        let expected = labels
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>();

        data.iter().find_map(|(metric, metric_labels, value)| {
            if metric == name && *metric_labels == expected {
                if let DebugValue::Gauge(v) = value {
                    return Some(v.into_inner());
                }
            }
            None
        })
    }

    pub fn counter_value(
        data: &[(String, BTreeMap<String, String>, DebugValue)],
        name: &str,
        labels: &[(&str, &str)],
    ) -> Option<u64> {
        let expected = labels
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>();

        data.iter().find_map(|(metric, metric_labels, value)| {
            if metric == name && *metric_labels == expected {
                if let DebugValue::Counter(v) = value {
                    return Some(*v);
                }
            }
            None
        })
    }
}
