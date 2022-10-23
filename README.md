# ESP32-C3 / S3 Espressif IDF OTA Experiment

## About

This experiment uses IDF OTA functionality in Rust on an ESP32-C3-Rust Board

This is WIP - should lead to a sensor node for an anemometer.


## Scope
### Technical
- relayable wifi connection, automatic reconnect
- MQTT transport of sensor data
- OTA update

### Functional
- HTML page for OTA update
- HTML page for providing current wind speed and direction
- Data feed to AWS where an external web interface is hosted
- WIFI parameter configuration through bluetooth

## What is working
- Reliable wifi re-connect. When the wifi connection gets dropped, a re-connection process is started. When an IP address is received the HTTP Server is started again.

The current OTA code is inspired by https://github.com/bakery/rust-esp32-std-demo/tree/feature/ota-updates




## Preparation

First copy `cfg.toml.example` to `cfg.toml` and configure SSID and PWD of your WiFi access point.
Your dev PC needs to be connected to the same access point.

Change the address in `html/ota-update.html` to your computers address

Change the version number in `Cargo.toml` to `0.1.0`

build and flash the solution

Change the version number in `Cargo.toml` and in `release.json` to `0.2.0`




## Run

Run `start_ws.py` in a separate terminal (in the project directory). 

To avoid problems run `esptool erase_flash` first. Now run the application via `cargo run --release`

The application should connect to your PC, pick up `current.txt` and see it's own version (1) is below what is available online (2).
Now it will download `firmware.bin` and flash it. After that it will set the OTA partition to use.

In this experiment the reset isn't done automatically. Reset the ESP32-C3 and see the new version boot.
The new version will see there is no later version online to flash.
