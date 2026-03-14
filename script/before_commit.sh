#!/bin/bash
set -e

./script/precheck.sh
cargo bench
./script/record_build_size.sh
