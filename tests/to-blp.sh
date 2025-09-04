#!/bin/bash
set -e

ROOT="../test-data/convert"

A="$ROOT/с.png"
B="$ROOT/с.blp"

cargo build --release

RUST_BACKTRACE=full ./../target/release/blp_rs to-blp "$A" "$B"

./../target/release/blp_rs "$B"
