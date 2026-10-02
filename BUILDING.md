# Building

## Requirements (developers only — players need nothing)
* Rust 1.85+ (`rustup`), a GPU with Metal / Vulkan / DX12.
* macOS: Xcode Command Line Tools (for `codesign`, `iconutil`, `hdiutil`).
* Linux: `libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev`.

## Common commands
```bash
cargo run --release -p cosmogon                      # the app
cargo run -p cosmogon --features dev                 # faster incremental builds (dynamic linking; not distributable)
cargo test -p cosmogon_core -p cosmogon_physics -p cosmogon_sim
cargo run --release -p cosmogon_cli -- run --scenario sol --seed 4 --years 202000
cargo run --release -p cosmogon_cli -- check --scenario garden --seed 9 --years 3000000   # determinism
cargo run --release -p cosmogon_cli -- survey --scenario garden --life hopeful --seeds 8 --years 2000000000
```

## Developer flags for the app
`--new sol|garden|neighbourhood --seed N --life 0|1|2 --advance-years Y --focus NAME --distance RADII
--yaw RAD --pitch RAD --speed INDEX --select-civ --hide-ui --load PATH --capture FILE.png --capture-after FRAMES --quit`

## Packaging
* **macOS:** `scripts/package-macos.sh` → `dist/Cosmogon.app` + `dist/Cosmogon-<v>-macOS.dmg`
  (ad-hoc signed). `UNIVERSAL=1` builds arm64 + x86_64. For public distribution set
  `SIGN_ID="Developer ID Application: …"`, then notarise:
  `xcrun notarytool submit dist/*.dmg --apple-id … --team-id … --wait && xcrun stapler staple dist/*.dmg`.
* **Windows:** built in CI (`release.yml`): portable zip and an Inno Setup installer
  (`packaging/windows/cosmogon.iss`); the icon is embedded by `crates/cosmogon/build.rs`. Release builds have
  no console window.
* **Linux:** tarball in CI; AppImage/Flatpak planned.
* Icon: regenerate with `python3 scripts/make_icon.py` (original artwork, stdlib only).

Everything the app needs (shaders, data files, fonts) is embedded in the binary.

## Where things are stored
Saves: `~/Library/Application Support/Cosmogon/saves` (macOS), `%APPDATA%\Cosmogon\saves` (Windows),
`~/.local/share/Cosmogon/saves` (Linux). Settings: the platform config directory, `Cosmogon/settings.toml`.
