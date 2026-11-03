# Сборка плагина: онлайн и офлайн

## 1. Что собирается без интернета

`src/dsp/**` — DSP-ядро плагина — **не зависит ни от одного внешнего крейта**
(только `std`). Поэтому его можно компилировать, тестировать и даже рендерить
звук в полностью закрытой сети:

```bash
cd tools/dsp_standalone
cargo test             # 43 юнит-теста DSP
cargo run --release    # рендер 8 с «супер атмосферы» в output/*.wav
python3 ../../tools/analyze_bands.py ../../output/post_ambient_super_atmospheric.wav
```

## 2. Полная сборка плагина (VST3) требует crates.io

`Cargo.toml` тянет `nih-plug` + `egui` (+ `winit`, `glutin` и прочее) — это
порядка 200 крейтов. В закрытой песочнице `cargo build` падает:

```
failed to download from `https://index.crates.io/config.json`
Caused by: [35] SSL connect error
```

На машине с доступом в интернет всё собирается без правок:

```bash
rustc --version        # 1.88.0
cargo build --release
# → target/release/post_ambient_vst.vst3 (Linux/macOS), *.dll (Windows)
```

## 3. Офлайн-сборка через vendoring

Если целевая машина без интернета, а сборка нужна полностью, — готовим
vendor-каталог **один раз** на машине с сетью и переносим его вместе с кодом.

### Шаг 1. На машине с интернетом

```bash
# фиксируем версии всех крейтов
cargo generate-lockfile          # создаст Cargo.lock

# выкачиваем исходники всех зависимостей в vendor/
cargo vendor --versioned-dirs vendor > .cargo/vendor-config.toml
```

### Шаг 2. `.cargo/config.toml` в корне репозитория

```toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
```

(`cargo vendor` сам печатает нужный блок — можно взять вывод
`.cargo/vendor-config.toml` и вставить его в `.cargo/config.toml`.)

### Шаг 3. На машине без интернета

```bash
cargo build --release --offline
cargo test  --offline
```

Размер `vendor/` для `nih-plug + egui` — порядка 150–250 МБ, поэтому в Git
его обычно **не** коммитят (см. `.gitignore`), а переносят архивом или
кэшируют в CI.

### Альтернатива: кэш cargo

```bash
CARGO_HOME=/path/to/cargo-home cargo fetch        # на машине с сетью
CARGO_HOME=/path/to/cargo-home cargo build --release --offline
```

## 4. Почему здесь нет `Cargo.lock`

`Cargo.lock` генерируется из индекса `crates.io`, который в этой песочнице
недоступен (`index.crates.io` закрыт), — сгенерировать корректный лок-файл
здесь невозможно, только вручную или на машине с сетью. Для `cdylib`-плагина
лок-файл к тому же не обязателен, но для воспроизводимых сборок его стоит
создать по шагу 1 и закоммитить.

## 5. Установка тулчейна без static.rust-lang.org

```bash
./scripts/install_rust_offline.sh
```

Скрипт берёт готовые бинарники `rustc`/`cargo`/`rust-std` из npm-реестра
(пакеты `@rustbin/*`), поэтому работает, даже когда `sh.rustup.rs` и
`static.rust-lang.org` заблокированы (см. README, раздел «🦀 Разработка на Rust»).
