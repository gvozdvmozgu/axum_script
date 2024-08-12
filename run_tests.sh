#!/usr/bin/env bash
set -e


cargo run tests/ &
RSPID=$!
trap "kill $RSPID" EXIT

while ! nc -z localhost 4000; do   
  sleep 0.2 
done

deno test --allow-net tests/test.js