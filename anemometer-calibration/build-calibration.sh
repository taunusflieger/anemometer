#!/bin/sh
. ~/export-esp.sh
. ~/esp_set_wifi.sh
cargo  espflash flash --monitor --features calibration --target xtensa-esp32s3-espidf  --port /dev/ttyACM0