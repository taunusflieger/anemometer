#!/bin/sh
while :
do
    echo "send request"
    curl --connect-timeout 2.0 --max-time 3.0 http://192.168.100.197 >/dev/null
    res=$?
    if test "$res" != "0"; then
        echo "ERROR retriving data the curl command failed with: $res"
    fi
    sleep 30
done

