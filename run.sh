#!/usr/bin/env bash
set -e

cd "$(dirname "$0")"

cargo build

mkdir -p plugins
cp target/debug/libplugin.so plugins/

cargo run -p host
