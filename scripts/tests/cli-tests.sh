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

t() {
  set +e
  RG_OUT=$(rg "$@")
  RX_OUT=$(rx "$@")
  set -e

  if [[ "$RG_OUT" != "$RX_OUT" ]]; then
    cat <<EOF
fail!

expected:
$RG_OUT

actual:
$RX_OUT
EOF
  fi
}

main() {
  build

  t foo <(echo 'foobar baz')
  t foobar <(echo '    foobar')
  t foo <(echo 'bar')
  echo 'foo' | rx fo '-'
  rx 'f(?<wut>o){2}' <(echo 'afoobar')
  t -e 'foo' -e 'bar' <(echo 'afoobar')
  FILE=$(mktemp) && echo 'foobar' > "$FILE" && t foo "$FILE"
  DIR=$(mktemp -d) && echo $'foo\nfoobar\nbarfoo' > "$DIR/file1" && echo 'barbaz' > "$DIR/file2" && t '(foo|bar)' "$DIR/file1" "$DIR/file2"
  DIR=$(mktemp -d) && echo $'foo\nfoobar\nbarfoo' > "$DIR/file1" && echo 'barbaz' > "$DIR/file2" && t '(foo|bar)' "$DIR"
  t 'hello(w?)world' <(echo 'helloworld')
}

main "$@"