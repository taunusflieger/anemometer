#!/bin/sh
esptool.py \
    --before default_reset \
    --after hard_reset \
    --chip esp32s3 \
    write_flash --flash_mode dio \
    --flash_freq 40m \
    --flash_size detect \
    0x790000 ./nvs/conf.bin