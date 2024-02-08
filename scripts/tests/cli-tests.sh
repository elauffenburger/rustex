#!/usr/bin/env bash
set -eu -o pipefail

_TEST_DIR="$(dirname "$0")"

build() {
  pushd "$_TEST_DIR/../../" &>/dev/null
  trap "popd" ERR

  cargo build

  popd &>/dev/null
}

cli() {
  "$_TEST_DIR/../../target/debug/rustex-cli" "$@"
}

main() {
  build

  echo 'foobar' | cli foo
  echo 'bar' | cli foo
  echo 'afoobar' | cli 'f(?<wut>o){2}'
  echo 'afoobar' | cli -e 'foo' -e 'bar'
}

main "$@"