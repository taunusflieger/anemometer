#!/bin/sh
version=$(sed -n 's/^version = //p' Cargo.toml | tr -d '"')
echo building firmware-$version.bin
cargo +esp espflash  save-image --chip esp32s3 --target xtensa-esp32s3-espidf -Zbuild-std=std,panic_abort  --flash-size 2mb ./bin/firmware-$version.bin
aws s3api put-object --bucket anemometer-fw-store --key firmware-$version.bin --body ./bin/firmware-$version.bin
rm ./bin/firmware-$version.bin