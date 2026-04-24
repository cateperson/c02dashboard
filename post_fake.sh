#!/usr/bin/env bash
set -euo pipefail

PORT=${PORT:-3000}
URL="http://127.0.0.1:${PORT}/data"
INTERVAL=${INTERVAL:-5}

while true; do
  co2=$(awk 'BEGIN { srand(); printf "%.4f", 0.040 + rand() * 0.020 }')
  time=$(date +%s.%N)

  status=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$URL" \
    -H "Content-Type: application/json" \
    -d "{\"co2\":${co2},\"time\":${time}}")

  echo "$(date '+%H:%M:%S')  co2=${co2}  → ${status}"
  sleep "$INTERVAL"
done
