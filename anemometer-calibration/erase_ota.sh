#!/bin/sh
~/esp/esp-idf/components/esptool_py/esptool/esptool.py --chip esp32s3 erase_region 0x190000 0x180000 
