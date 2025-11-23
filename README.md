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

### Bare Metal
Build and run it directly on the desired host

```shell
cargo build --release
PORT=9185 TIMEOUT=60s ./target/release/ruuvi-prometheus-rs
```

### Container Image
The exporter can be run inside a container, but the host must have a running `bluethooth.service`
just like for the bare metal case. Additionally, during runtime the D-Bus socket must be mounted
into the container.

```shell
docker build -t <image-name> .
docker run -v /run/dbus:/run/dbus:ro -p 9185:9185 ... <image-name>
```
Depending on your setup the `--privileged` might be needed. A pre-build image
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
if only specific hosts have a bluetooth device, taints and tolerations must be used to ensure
the pod is scheduled on the correct host.

Theoretically we could install all of the bluetooth stack into the container and then mount the
bluetooth device directly. However, this has two downsides

- The container image will become more complex and harder to maintain
- Only a single process can bind to the underlying hardware device, so as soon as this
  hypothetical container runs no other service could use any bluethooth functionality
  on that host


## Environment variables

| Variable            | Description                                     | Default         |
|---------------------|-------------------------------------------------|-----------------|
| `PORT`              | Port to listen on for the metrics endpoint      | 9185            |
| `IDLE_TIMEOUT`      | Idle timeout for metrics                        | 60s             |
| `BLUETOOTH_DEVICE`  | Which bluetooth device to use (e.g. hci0)       | hci0            |
