#!/bin/sh
~/esp/esp-idf/components/nvs_flash/nvs_partition_generator/nvs_partition_gen.py generate "../../weatherStationSecrets/$1/nvs.csv" conf.bin 0x10000 --outdir ./nvs