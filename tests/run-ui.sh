#!/bin/bash
set -e

A="../test-data/scan/BLP1_tt0_c0_ab8_at0_m0_512x64/Loading-BarGlass.blp"
A="../test-data/scan/BLP1_tt1_c0_ab8_at0_m0_512x128/CenterPanel01.blp"

cargo build --release

./../target/release/blp_rs "$A"
