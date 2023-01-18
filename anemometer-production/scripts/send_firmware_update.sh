#!/bin/sh
aws iot-data publish --topic arn:aws:iot:eu-west-1:102167871435:thing/$1/command/ota_update  --cli-binary-format raw-in-base64-out --payload $2