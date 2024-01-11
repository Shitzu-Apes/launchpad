#!/bin/bash
set -e

mkdir -p ./res

cargo build -p skyward --target wasm32-unknown-unknown --release
cargo build -p permissions --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/*.wasm ./res/

wasm-opt -O4 res/skyward.wasm -o res/skyward.wasm --strip-debug --vacuum
wasm-opt -O4 res/permissions.wasm -o res/permissions.wasm --strip-debug --vacuum
