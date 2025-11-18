# Ruuvi Exporter

Listen to BLE advertisements of Ruuvi tags. Supports [v5](https://docs.ruuvi.com/communication/bluetooth-advertisements/data-format-5-rawv2), [v6](https://docs.ruuvi.com/communication/bluetooth-advertisements/data-format-6) and [E1](https://docs.ruuvi.com/communication/bluetooth-advertisements/data-format-e1) of the Ruuvi protocol so far.

## Exposed metrics

| Metric                        | v5 | v6 | E1 |
|-------------------------------|----|----|----|
| Temperature (°C)              | ✔️ | ✔️ | ✔️ |
| Humidity (%RH)                | ✔️ | ✔️ | ✔️ |
| Dew Point (°C)                | ✔️ | ✔️ | ✔️ |
| Pressure (hPa)                | ✔️ | ✔️ | ✔️ |
| Acceleration (g)              | ✔️ | ✗ | ✗ |
| Battery Voltage (mV)          | ✔️ | ✗ | ✗ |
| Move Counter                  | ✔️ | ✗ | ✗ |
| PM 1.0 (ug/m³)                | ✗ | ✗ | ✔️ |
| PM 2.5 (ug/m³)                | ✗ | ✔️ | ✔️ |
| PM 4.0 (ug/m³)                | ✗ | ✗ | ✔️ |
| PM 10.0 (ug/m³)               | ✗ | ✗ | ✔️ |
| CO_2 (ppm)                    | ✗ | ✔️ | ✔️ |
| VOC index                     | ✗ | ✔️ | ✔️ |
| NO_x index                    | ✗ | ✔️ | ✔️ |
| Air quality calibrating       | ✗ | ✔️ | ✔️ |
| Signal Strength, rssi (dBm)   | ✔️ | ✔️ | ✔️ |
| Transmitting Strength (dBm)   | ✔️ | ✔️ | ✔️ |
| Last Updated                  | ✔️ | ✔️ | ✔️ |
| Format                        | ✔️ | ✔️ | ✔️ |
| Messages Received             | ✔️ | ✔️ | ✔️ |


# How to run

## Experimental bluetooth features

```shell
sudo nano /usr/lib/systemd/system/bluetooth.service
...
sudo systemctl daemon-reload
sudo service bluetooth restart
```

```
ExecStart=/usr/libexec/bluetooth/bluetoothd --experimental
```

## Build & Run

Since building a docker image with the correct bluetooth dependencies installed is tricky, so far only running directly on the host is supported.

```shell
cargo build --release
PORT=9185 TIMEOUT=60s ./target/release/ruuvi-prometheus-rs
```

## Environment variables

| Variable            | Description                                     | Default         |
|---------------------|-------------------------------------------------|-----------------|
| `PORT`              | Port to listen on for the metrics endpoint      | 9185            |
| `IDLE_TIMEOUT`      | Idle timeout for metrics                        | 60s             |
| `BLUETOOTH_DEVICE`  | Which bluetooth device to use (e.g. hci0)       | hci0            |
