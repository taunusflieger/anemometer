#!/bin/sh
version=$(sed -n 's/^version = //p' Cargo.toml | tr -d '"')
cargo espflash  save-image --release --chip esp32c3 --target riscv32imc-esp-espidf -Zbuild-std=std,panic_abort  --flash-size 2M ./bin/firmware-$version.bin
