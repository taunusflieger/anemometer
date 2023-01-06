#!/bin/sh
docker run -it hivemq/mqtt-cli pub -i mypub -t anemometer/command/system_restart -h 192.168.100.86 -m:empty 