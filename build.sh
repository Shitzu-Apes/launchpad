#!/bin/bash
set -e

mkdir -p ./res

cargo build -p skyward --target wasm32-unknown-unknown --release
cargo build -p permissions --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/*.wasm ./res/

cargo build -p skyward --target wasm32-unknown-unknown --features=integration-test --release
cp target/wasm32-unknown-unknown/release/skyward.wasm ./res/skyward_testing.wasm

wasm-opt -O4 res/skyward.wasm -o res/skyward.wasm --strip-debug --vacuum
wasm-opt -O4 res/skyward_testing.wasm -o res/skyward_testing.wasm --strip-debug --vacuum
wasm-opt -O4 res/permissions.wasm -o res/permissions.wasm --strip-debug --vacuum
