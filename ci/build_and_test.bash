#!/bin/bash

set -eu

target=$1
shift

if [[ "$target" == aarch64-unknown-linux-* ]]; then
  apt-get update -y
  apt-get install -y gcc-aarch64-linux-gnu
  export QEMU_LD_PREFIX=/usr/aarch64-linux-gnu/
  export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
fi

rustup target add "$target"

cargo build --target="$target" "$@"
cargo test --target="$target" "$@"
