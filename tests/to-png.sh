#!/bin/bash
set -e

C="to-png"

ROOT="../test-data/$C"

A="$ROOT/a.blp"
B="$ROOT/a.png"

cargo build --release

RUST_BACKTRACE=full ./../target/release/blp_rs "$C" "$A" "$B"

./../target/release/blp_rs "$B"
