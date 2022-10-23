#!/bin/sh
version=$(sed -n 's/^version = //p' Cargo.toml | tr -d '"')
cargo espflash --partition-table ./partitions.csv save-image ./bin/firmware-$version.bin
