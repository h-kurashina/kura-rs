#!/usr/bin/env bash
# Vercel のビルド環境に Rust を入れてから npm の依存を入れる。
# サイトのビルド前に kura-registry（Rust）で registry/*.json を検証・書き出すため。
# site/vercel.json の installCommand から呼ばれる（作業ディレクトリは site/）。
set -euo pipefail

# Rust のリンクに C コンパイラが要る
if ! command -v cc >/dev/null 2>&1; then
  dnf install -y gcc
fi

# Vercel の環境には最初から cargo が入っていることがあるが、版が合わないので使わない。
# rustup を入れて、リポジトリ直下の rust-toolchain.toml の版を使う
if [ ! -x "$HOME/.cargo/bin/rustup" ]; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | RUSTUP_INIT_SKIP_PATH_CHECK=yes sh -s -- -y --profile minimal --default-toolchain none --no-modify-path
fi

npm ci
