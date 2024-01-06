# rustex

## Overview

`rustex` is a native Rust implementation of regular expressions. It's not PCRE, Perl, or JS flavored; in fact, if it had a flavor it probably wouldn't taste all that good :)

## Features

  - [x] Words
  - [x] `^` and `$`
  - Sets
    - [x] `[abc123]`
    - [x] `[^abc123]`
  - Repetition
    - [x] `hello{1}`
    - [x] `hello{1,5}`
    - [x] `hello{1,}`
