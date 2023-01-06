#!/bin/sh
docker run -it hivemq/mqtt-cli pub -i mypub -t anemometer/command/ota_update -m http://192.168.100.86/bin/firmware-0.1.1.bin -h 192.168.100.86 