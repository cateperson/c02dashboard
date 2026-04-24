#!/usr/bin/env bash
set -euo pipefail

if [ -z "${1:-}" ]; then
  echo "Error: No CO2 value provided."
  exit 1
fi

CO2_VALUE=$1
PORT=${PORT:-3000}
URL="http://127.0.0.1:${PORT}/data"
TIME=$(date +%s.%N)

status=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST "$URL" \
  -H "Content-Type: application/json" \
  -d "{\"co2\":${CO2_VALUE},\"time\":${TIME}}")

echo "Status: ${status} | CO2: ${CO2_VALUE} | Time: ${TIME}"

if [ "$status" != "201" ]; then
  exit 1
fi
