#!/bin/sh
# One series of the width scan of node V22 (PREDECLARED.json).
# usage: run_scan.sh <runner binary> <index> <tag>
set -eu
runner=$1
index=$2
tag=$3
here=$(dirname "$0")
# Attempts of the long run by half band width (PREDECLARED.json).
attempts() {
    if [ "$1" -le 8 ]; then echo 60
    elif [ "$1" -le 16 ]; then echo 40
    elif [ "$1" -le 32 ]; then echo 24
    else echo 12
    fi
}
for w in 1 2 3 4 5 6 8 10 12 16 24 32 64; do
    python3 -I "$here/measure.py" "$runner" acstrip "$w" "$(attempts "$w")" "$index" --tag "$tag"
done
for m in 1 2 3 4 6 8 12 16 32; do
    python3 -I "$here/measure.py" "$runner" brstrip "$m" "$(attempts $((2 * m)))" "$index" --tag "$tag"
done
