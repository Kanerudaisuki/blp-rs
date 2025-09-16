#!/bin/bash
set -e

A=../test-data/scan/BLP1_tt0_c0_ab8_at0_m0_512x64/Loading-BarGlass.blp
A=../test-data/scan/BLP1_tt1_c0_ab8_at0_m0_512x128/CenterPanel01.blp
#A=~/Downloads/PSD/272280-f1c6ea1c7e5aac25781dab8c5798361e.psd

cargo build --release

#./../target/release/blp_rs "$A"
./../target/release/blp_rs
