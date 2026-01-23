# Ruuvi Exporter

Listen to BLE advertisements of Ruuvi tags. Supports [v5](https://docs.ruuvi.com/communication/bluetooth-advertisements/data-format-5-rawv2), [v6](https://docs.ruuvi.com/communication/bluetooth-advertisements/data-format-6) and [E1](https://docs.ruuvi.com/communication/bluetooth-advertisements/data-format-e1) of the Ruuvi protocol so far.

## Exposed metrics

| Metric                      | Description                   | v5 | v6 | E1 |
|-----------------------------|-------------------------------|----|----|----|
| `ruuvi_temperature_celsius` | Temperature (°C)              | ✔️ | ✔️ | ✔️ |
| `ruuvi_humidity_ratio`      | Humidity (%RH)                | ✔️ | ✔️ | ✔️ |
| `ruuvi_dew_point_celsius`   | Dew Point (°C)                | ✔️ | ✔️ | ✔️ |
| `ruuvi_pressure_hpa`        | Pressure (hPa)                | ✔️ | ✔️ | ✔️ |
| `ruuvi_rssi_dbm`            | Signal Strength, rssi (dBm)   | ✔️ | ✔️ | ✔️ |
| `ruuvi_last_updated`        | Last Updated                  | ✔️ | ✔️ | ✔️ |
| `ruuvi_frames_total`        | Messages Received             | ✔️ | ✔️ | ✔️ |
| `ruuvi_acceleration_g`      | Acceleration (g)              | ✔️ | ✗ | ✗ |
| `ruuvi_battery_volts`       | Battery Voltage (V)           | ✔️ | ✗ | ✗ |
| `ruuvi_txpower_dbm`         | Transmitting Strength (dBm)   | ✔️ | ✗ | ✗ |
| `ruuvi_movecount_total`     | Move Counter                  | ✔️ | ✗ | ✗ |
| `ruuvi_pm1_0_ug_m3`         | PM 1.0 (ug/m³)                | ✗ | ✗ | ✔️ |
| `ruuvi_pm2_5_ug_m3`         | PM 2.5 (ug/m³)                | ✗ | ✔️ | ✔️ |
| `ruuvi_pm4_0_ug_m3`         | PM 4.0 (ug/m³)                | ✗ | ✗ | ✔️ |
| `ruuvi_pm10_0_ug_m3`        | PM 10.0 (ug/m³)               | ✗ | ✗ | ✔️ |
| `ruuvi_co2_ppm`             | CO_2 (ppm)                    | ✗ | ✔️ | ✔️ |
| `ruuvi_voc_index`           | VOC index                     | ✗ | ✔️ | ✔️ |
| `ruuvi_nox_index`           | NO_x index                    | ✗ | ✔️ | ✔️ |
| `ruuvi_air_quality_index`   | Air quality index             | ✗ | ✔️ | ✔️ |
| `ruuvi_air_calibrating`     | Air quality calibrating       | ✗ | ✔️ | ✔️ |

Optionally, some process metrics can also being published, if enabled via environment variable. This can be helpful when running on bare metal, but is usually not needed if running in a container where container/process metrics are being collected via other mechanisms:

| Metric                             | Description                                                                     |
|------------------------------------|---------------------------------------------------------------------------------|
| `process_cpu_seconds_total`        | Total user and system CPU time spent in seconds.                                |
| `process_open_fds`                 | Number of open file descriptors.                                                |
| `process_max_fds`                  | Maximum number of open file descriptors.                                        |
| `process_virtual_memory_bytes`     | Virtual memory size in bytes.                                                   |
| `process_virtual_memory_max_bytes` | Maximum amount of virtual memory available in bytes. (Not available on Windows) |
| `process_resident_memory_bytes`    | Resident memory size in bytes.                                                  |
| `process_start_time_seconds`       | Start time of the process since unix epoch in seconds.                          |
| `process_threads`                  | Number of OS threads in the process. (Not available on Windows)                 |


# How to run

## Experimental bluetooth features
Needed to be able to connect to the bluetooth service as an unprivileged user.

```shell
sudo nano /usr/lib/systemd/system/bluetooth.service
...
sudo systemctl daemon-reload
sudo service bluetooth restart
```

Modify the following line:
```
ExecStart=/usr/libexec/bluetooth/bluetoothd --experimental
```

## Environment variables

| Variable                      | Description                                       | Default         |
|-------------------------------|---------------------------------------------------|-----------------|
| `PORT`                        | Port to listen on for the metrics endpoint        | 9185            |
| `IDLE_TIMEOUT`                | Idle timeout for metric to be removed             | 60s             |
| `ENABLE_PROCESS_COLLECTION`   | Enable process metrics                            | false           |
| `PROCESS_COLLECTION_INTERVAL` | Interval with which process metrics are collected | 10s             |
| `BLUETOOTH_DEVICE`            | Which bluetooth device to use (e.g. hci0)         | hci0            |


## Build & Run

### Bare Metal
Build and run it directly on the desired host

```shell
cargo build --release
ENABLE_PROCESS_COLLECTION=true ./target/release/ruuvi-prometheus-rs
```

### Container Image
The exporter can be run inside a container, but the host must have a running `bluethooth.service`
just like for the bare metal case. Additionally, during runtime the D-Bus socket must be mounted
into the container.

```shell
docker build -t <image-name> .
docker run -v /run/dbus:/run/dbus:ro -p 9185:9185 ... <image-name>
```
Depending on your setup the `--privileged` flag might be needed. A pre-built image
is published with every release, and is available via `ghcr.io/graipher/ruuvi-prometheus-rs:vX.Y.Z`.

Similarly, it can be run inside a Kubernetes cluster
```yaml
apiVersion: v1
kind: Pod
metadata:
  name: ruuvi-prometheus-rs
spec:
  containers:
    - name: ruuvi
      image: ghcr.io/graipher/ruuvi-prometheus-rs:vX.Y.Z
      imagePullPolicy: IfNotPresent
      ports:
        - containerPort: 9185
      volumeMounts:
        - mountPath: "/run/dbus"
          name: "dbus-socket"
          readOnly: true
  volumes:
    - name: "dbus-socket"
      hostPath:
        path: "/run/dbus"
```

If only specific hosts have a bluetooth device, taints and tolerations must be used to ensure
the pod is scheduled on the correct host.

Theoretically we could install all of the bluetooth stack into the container and then mount the
bluetooth device directly. However, this has two downsides

- The container image will become more complex and harder to maintain
- Only a single process can bind to the underlying hardware device, so as soon as this
  hypothetical container runs no other service could use any bluethooth functionality
  on that host
