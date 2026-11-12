#!/bin/sh
# Serves the web build on http://localhost:8642/ (loopback only). Build first with tools/web/build.sh.
#
#   tools/web/serve.sh                       then open the page and choose your GTA San Andreas folder
#   tools/web/serve.sh --game DIR            also serve DIR (your own install) to this machine, no picker needed
#   tools/web/serve.sh --port N
set -eu
exec python3 -I "$(dirname "$0")/serve.py" "$@"
