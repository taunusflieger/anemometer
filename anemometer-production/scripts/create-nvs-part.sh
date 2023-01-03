#!/bin/sh
~/esp/esp-idf/components/nvs_flash/nvs_partition_generator/nvs_partition_gen.py generate "./nvs.csv" certs.bin 16384 --outdir ./nvs