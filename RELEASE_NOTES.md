# Cosmogon 0.3.0 — "Living worlds, rival nations"

A free, open-source simulator of a living universe: stars, planets, life and civilizations
that discover technology through causes rather than timers.

## Downloads
| Platform | File | Notes |
|---|---|---|
| macOS 11+ (Apple Silicon & Intel) | `Cosmogon-0.3.0-macOS.dmg` | Open, drag Cosmogon to Applications. |
| Windows 10/11 (64-bit) | `Cosmogon-0.3.0-Windows-Setup.exe` | Installer with Start-menu shortcut. |
| Windows 10/11 (64-bit) | `Cosmogon-0.3.0-Windows.exe` | Portable: just run it. |
| Linux x86-64 | `Cosmogon-linux-x86_64.tar.gz` | Extract and run `cosmogon`. |

Nothing else needs to be installed. A GPU with Metal, Vulkan or DirectX 12 is required.

### First launch (unsigned builds)
These builds are not signed with paid Apple/Microsoft certificates, so the operating system
warns the first time:
* **macOS:** right-click Cosmogon → **Open** → **Open**. If macOS says the app "is damaged",
  run `xattr -dr com.apple.quarantine /Applications/Cosmogon.app` in Terminal once.
  On macOS 15+, you may instead need System Settings → Privacy & Security → **Open Anyway**.
* **Windows:** SmartScreen → **More info** → **Run anyway**.

## What's new
* **Real Earth.** The Sol scenario uses NOAA ETOPO5 relief: real continents, mountains and sea
  floors. Humanity begins in East Africa and spreads by walking, rafts and ships.
* **Rival nations.** Chiefdoms, kingdoms, empires, republics and federations form from
  geography, fight border wars, capture cities, unite, fracture in civil wars — and may, rarely,
  become a world government. Territories are coloured on the map.
* **Close-up terrain.** Fly down to a few hundred metres above any rocky world: a level-of-detail
  quadtree of real 3D relief with mountain ranges, crater fields on airless worlds and haze.
* **Geological speed with civilizations.** Societies are simulated at the level of detail they
  need, so millions of years per second stay possible after intelligence appears.
* **Same universe everywhere.** A seed now produces bit-identical results on every computer and
  operating system (verified in CI on macOS, Windows and Linux).
* Fixes: airless worlds were drawn black; Mars and other dry worlds were drawn as ice.
