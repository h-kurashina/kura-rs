#!/usr/bin/env bash
# Vercel の buildCommand から呼ばれる（作業ディレクトリは site/）。
# vercel-install.sh で入れた rustup の cargo を、最初から入っている cargo より優先して使う。
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"
cargo --version
npm run build
