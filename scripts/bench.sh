#!/usr/bin/env bash
set -euo pipefail

mode="${1:-quick}"
baseline="${2:-}"

run_bench() {
  cargo bench --bench benchmark -- "$@"
}

case "$mode" in
  quick)
    run_bench --quick --noplot
    ;;
  full)
    run_bench --noplot
    ;;
  save-baseline)
    if [[ -z "$baseline" ]]; then
      echo "usage: scripts/bench.sh save-baseline <name>" >&2
      exit 1
    fi
    run_bench --noplot --save-baseline "$baseline"
    ;;
  compare)
    if [[ -z "$baseline" ]]; then
      echo "usage: scripts/bench.sh compare <name>" >&2
      exit 1
    fi
    run_bench --noplot --baseline "$baseline"
    ;;
  list)
    run_bench --list
    ;;
  *)
    echo "usage: scripts/bench.sh [quick|full|save-baseline <name>|compare <name>|list]" >&2
    exit 1
    ;;
esac
