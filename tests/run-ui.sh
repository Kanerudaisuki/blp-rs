#!/bin/bash
set -e

FILE="/Users/nazarpunk/Downloads/_bg/10.png"
FILE="../test-data/scan/BLP1_tt1_c0_ab8_at0_m0_512x512.blp"
FILE="../test-data/scan/BLP1_tt1_c0_ab8_at0_m0_512x512_512x512.png"

# Сборка в release для скорости запуска
cargo build --release

# Запуск GUI с тест-файлом
./../target/release/blp_rs "$FILE"
