# Ruuvi Exporter

Listen to BLE advertisements of Ruuvi tags. Supports [v5](https://docs.ruuvi.com/communication/bluetooth-advertisements/data-format-5-rawv2) of the protocol so far.

## Exposed metrics

Output temperature (in °C), humidity (in %RH), dew point (in °C), pressure (in hPa), acceleration (in g), battery voltage (in mV), signal strength (in dBm), last updated and number of messages received, per Ruuvi tag.

# How to run

Build and run with Docker:

```sh
docker build -t ruuvi-prometheus-rs .
docker run -it --rm \
    -e PORT=9185 \
    ruuvi-prometheus-rs
```
