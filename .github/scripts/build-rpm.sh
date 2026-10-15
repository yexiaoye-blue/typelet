#!/usr/bin/env bash
# 在 Fedora 44 容器内执行，生成与本机一致的 RPM。
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
  make \
  pkgconf-pkg-config \
  webkit2gtk4.1-devel \
  gtk3-devel \
  openssl-devel \
  librsvg2-devel \
  libxdo-devel \
  libappindicator-gtk3-devel \
  patchelf \
  rpm-build

node_version=24.19.0
curl -fsSL "https://nodejs.org/dist/v${node_version}/node-v${node_version}-linux-x64.tar.xz" \
  | tar -xJ -C /usr/local --strip-components=1

export PATH="${CARGO_HOME}/bin:${PATH}"
if [[ ! -x "${CARGO_HOME}/bin/rustup" ]]; then
  curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal --no-modify-path
fi

npm ci
npx tauri build --bundles rpm
