#!/usr/bin/env bash
set -eu -o pipefail

_TEST_DIR="$(dirname "$0")"

build() {
  pushd "$_TEST_DIR/../../" &>/dev/null
  trap "popd" ERR

  cargo build

  popd &>/dev/null
}

rx() {
  "$_TEST_DIR/../../target/debug/rustex-cli" "$@"
}

main() {
  build

  echo 'foobar baz' | rx foo
  echo '    foobar' | rx foobar
  echo 'bar' | rx foo
  echo 'foo' | rx fo '-'
  echo 'afoobar' | rx 'f(?<wut>o){2}'
  echo 'afoobar' | rx -e 'foo' -e 'bar'
  FILE=$(mktemp) && echo 'foobar' > "$FILE" && rx foo "$FILE"
  DIR=$(mktemp -d) && echo $'foo\nfoobar\nbarfoo' > "$DIR/file1" && echo 'barbaz' > "$DIR/file2" && rx '(foo|bar)' "$DIR/file1" "$DIR/file2"
  DIR=$(mktemp -d) && echo $'foo\nfoobar\nbarfoo' > "$DIR/file1" && echo 'barbaz' > "$DIR/file2" && rx '(foo|bar)' "$DIR"
}

main "$@"