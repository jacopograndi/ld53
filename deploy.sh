#! /bin/bash

cargo build --target wasm32-unknown-unknown --release

wasm-bindgen --out-dir ./out/ --target web ./target/wasm32-unknown-unknown/release/ld53-anvil-express.wasm
# wasm-opt -Os out/wtanks_bg.wasm -o out/wtanks_bg.wasm
cp -r assets/ out/
zip -r out.zip out
butler push out.zip zjikra/anvil-express:wasm
