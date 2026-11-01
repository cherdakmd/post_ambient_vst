#!/usr/bin/env bash
# scripts/install_rust_offline.sh
#
# Установка rustc / cargo БЕЗ доступа к static.rust-lang.org и sh.rustup.rs
# (полезно в закрытых/ограниченных сетях: нужен только npm-реестр registry.npmjs.org).
#
# Идея: проект @rustbin публикует компоненты тулчейна Rust (rustc, cargo, rust-std)
# как обычные npm-пакеты, внутри которых лежат готовые бинарники. Мы скачиваем
# tarball'ы напрямую из npm-реестра и ставим их штатным rust-installer'ом.
#
# Использование:
#   ./scripts/install_rust_offline.sh                # ставит в /opt/rust (нужен sudo)
#   RUST_PREFIX=$HOME/rust ./scripts/install_rust_offline.sh
#
# После установки:
#   export PATH="$RUST_PREFIX/bin:$PATH"
#   rustc --version && cargo --version

set -euo pipefail

RUST_VERSION="${RUST_VERSION:-1.88.0}"
CARGO_PKG_VERSION="${CARGO_PKG_VERSION:-0.89.0}"   # версия npm-пакета cargo для RUST_VERSION
TRIPLE="${TRIPLE:-x86_64-unknown-linux-gnu}"
RUST_PREFIX="${RUST_PREFIX:-/opt/rust}"
REGISTRY="${REGISTRY:-https://registry.npmjs.org}"

pkg_name() {
  # $1 = компонент (rustc | cargo | rust-std), $2 = версия npm-пакета
  echo "@rustbin/$1-${RUST_VERSION}-${TRIPLE}/-/$1-${RUST_VERSION}-${TRIPLE}-$2.tgz"
}

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

mkdir -p "$RUST_PREFIX" 2>/dev/null || sudo mkdir -p "$RUST_PREFIX"

echo "==> Установка Rust ${RUST_VERSION} (${TRIPLE}) в ${RUST_PREFIX}"

download_and_install() {
  local component="$1" pkg_version="$2" pkg
  pkg="$(pkg_name "$component" "$pkg_version")"
  local tarball="${WORKDIR}/${component}.tgz"
  echo "--> скачивание ${component} (${pkg_version})"
  curl -fsSL -o "$tarball" "${REGISTRY}/${pkg}"
  echo "--> распаковка ${component}"
  mkdir -p "${WORKDIR}/${component}"
  tar xzf "$tarball" -C "${WORKDIR}/${component}"
  echo "--> установка ${component}"
  (cd "${WORKDIR}/${component}/package" && ./install.sh --prefix="${RUST_PREFIX}" --disable-ldconfig >/dev/null)
}

download_and_install rustc "${RUST_VERSION}"
download_and_install cargo "${CARGO_PKG_VERSION}"
download_and_install rust-std "${RUST_VERSION}"

# Симлинки в /usr/local/bin, чтобы rustc/cargo были в PATH
if [[ -w /usr/local/bin ]]; then
  ln -sf "${RUST_PREFIX}/bin/rustc" /usr/local/bin/rustc
  ln -sf "${RUST_PREFIX}/bin/cargo" /usr/local/bin/cargo
  ln -sf "${RUST_PREFIX}/bin/rustdoc" /usr/local/bin/rustdoc
elif command -v sudo >/dev/null 2>&1; then
  sudo ln -sf "${RUST_PREFIX}/bin/rustc" /usr/local/bin/rustc
  sudo ln -sf "${RUST_PREFIX}/bin/cargo" /usr/local/bin/cargo
  sudo ln -sf "${RUST_PREFIX}/bin/rustdoc" /usr/local/bin/rustdoc
fi

echo "==> Готово. Проверка:"
"${RUST_PREFIX}/bin/rustc" --version
"${RUST_PREFIX}/bin/cargo" --version

cat <<EOF

Добавьте в ~/.bashrc (или ~/.zshrc):
  export PATH="${RUST_PREFIX}/bin:\$PATH"

Проверка DSP-ядра плагина без внешних зависимостей:
  cd tools/dsp_standalone && cargo test && cargo run --release

Полная сборка плагина (нужен доступ к crates.io):
  cargo build --release
EOF
