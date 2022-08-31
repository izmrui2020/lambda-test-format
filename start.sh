#!/bin/bash

echo "start lambda test"
# cargo run --release -- \
#     --mode Series \
#     --out_path "./log" \
#     --tasks 10 \
#     --thread 2

cargo run --release -- \
    --mode Parallel \
    --out_path "./log" \
    --tasks 10 \
    --thread 4