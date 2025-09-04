#!/bin/bash
set -e

ROOT="../test-data/convert"

# FILE="$ROOT/BLP1_tt1_c0_ab8_at0_m0_512x512_512x512.png"

cargo build --release

./../target/release/blp_rs convert "$ROOT/BLP1_tt1_c0_ab8_at0_m0_512x512.blp" png "../test-data/convert/a.png"
