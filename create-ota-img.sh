#!/bin/sh
version=$(sed -n 's/^version = //p' Cargo.toml | tr -d '"')
cargo espflash  save-image --release ESP32-S3 --flash-size 2MB ./bin/firmware-$version.bin
