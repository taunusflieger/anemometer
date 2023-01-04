#!/bin/sh
esptool.py \
    --before default_reset \
    --after hard_reset \
    --chip esp32c3 \
    write_flash --flash_mode dio \
    --flash_freq 40m \
    --flash_size detect \
    0x3fd000 ./nvs/conf.bin