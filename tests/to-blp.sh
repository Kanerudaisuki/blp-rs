#!/bin/bash
set -e

C="to-blp"

ROOT="../test-data/$C"

A="$ROOT/a.png"
B="$ROOT/a.blp"

cargo build --release

RUST_BACKTRACE=full ./../target/release/blp_rs "$C" "$A" "$B"

./../target/release/blp_rs "$B"
