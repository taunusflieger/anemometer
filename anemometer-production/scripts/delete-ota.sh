#!/bin/sh
esptool.py --chip esp32s3 erase_region 0x290000 0x2000  
esptool.py --chip esp32s3 erase_region 0x510000 0x2000  