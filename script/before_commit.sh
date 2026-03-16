#!/bin/bash
set -e

./script/precheck.sh
cargo bench --bench benchmark
./script/record_build_size.sh
