#!/usr/bin/env bash
# Spin up / tear down a dummy HTTP server on port 3000
# Used as a prereq for 02-kill-port.tape
set -euo pipefail

PORT=3000

case "${1:-start}" in
  start)
    echo "Starting fake server on port $PORT ..."
    python3 -m http.server "$PORT" &>/dev/null &
    echo "PID: $!"
    ;;
  stop)
    echo "Stopping fake server on port $PORT ..."
    lsof -ti :"$PORT" | xargs kill -9 2>/dev/null || true
    ;;
  *)
    echo "Usage: $0 {start|stop}"
    exit 1
    ;;
esac
