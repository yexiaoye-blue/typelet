#!/usr/bin/env bash
# 在 Fedora 44 容器内执行，按容器架构生成 RPM。
set -euo pipefail

cd /work

dnf install -y \
  curl \
  wget \
  file \
  ca-certificates \
  xz \
  tar \
  gzip \
  gcc \
  gcc-c++ \
  clang \
  make \
  pkgconf-pkg-config \
  webkit2gtk4.1-devel \
  alsa-lib-devel \
  pipewire-devel \
  gtk3-devel \
  openssl-devel \
  librsvg2-devel \
  libxdo-devel \
  libappindicator-gtk3-devel \
  patchelf \
  rpm-build

node_version=24.19.0
case "$(uname -m)" in
  x86_64) node_arch=x64 ;;
  aarch64) node_arch=arm64 ;;
  *)
    echo "unsupported machine $(uname -m)" >&2
    exit 1
    ;;
esac
curl -fsSL "https://nodejs.org/dist/v${node_version}/node-v${node_version}-linux-${node_arch}.tar.xz" \
  | tar -xJ -C /usr/local --strip-components=1

cargo_home="${CARGO_HOME:-/work/.ci/cargo}"
export PATH="${cargo_home}/bin:${PATH}"
if [[ ! -x "${cargo_home}/bin/rustup" ]]; then
  curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal --no-modify-path
fi

npm ci
npx tauri build --bundles rpm
