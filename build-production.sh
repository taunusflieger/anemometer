#!/bin/sh
. ~/export-esp.sh
. ~/esp_set_wifi.sh
cargo  espflash flash --monitor --features production --target riscv32imc-esp-espidf  --port /dev/ttyACM0