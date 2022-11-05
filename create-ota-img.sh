#!/bin/sh
version=$(sed -n 's/^version = //p' Cargo.toml | tr -d '"')
cargo espflash  save-image --release --chip esp32s3 --merge --partition-table ./partitions.csv ./bin/firmware-$version.bin
