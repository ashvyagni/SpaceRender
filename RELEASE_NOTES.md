# Cosmogon 0.4.0 — "Sandbox Foundation"

A free, open-source simulator of a living universe: stars, planets, life and civilizations
that discover technology through causes rather than timers.

## Downloads
| Platform | File | Notes |
|---|---|---|
| macOS 11+ (Apple Silicon & Intel) | `Cosmogon-0.4.0-macOS.dmg` | Open, drag Cosmogon to Applications. |
| Windows 10/11 (64-bit) | `Cosmogon-0.4.0-Windows-Setup.exe` | Installer with Start-menu shortcut. |
| Windows 10/11 (64-bit) | `Cosmogon-0.4.0-Windows.exe` | Portable: just run it. |
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
* **A universe sandbox.** A new home screen with New Sandbox (seven templates), Continue, Load,
  Real Universe and Scenarios. Every experiment is saved in its own folder with autosave,
  checkpoints, duplicates and thumbnails.
* **The real Solar System, today.** The Solar System Lab starts from NASA/JPL Horizons positions
  and velocities for 1 January 2026. Run it for a year and the planets land within kilometres of
  where JPL says they will be.
* **Real gravity.** Dynamic N-body physics: add a planet and every other planet feels it.
  Collisions merge bodies; close encounters are integrated accurately; presets from Fast to Research.
* **Create and launch.** Add planets, moons, giants, dwarf planets, asteroids and comets; throw
  them at a target and see the predicted path — and any impact — before you release.
* **Edit anything, undo anything.** Mass, radius, gravity, day length, tilt, orbit, atmosphere,
  water, the Sun's mass and age — in your choice of units, with every value labelled MEASURED,
  DERIVED, ESTIMATED, PROCEDURAL or USER MODIFIED.
* **Consequences.** Move a planet or change the Sun and its climate, habitability and
  civilizations respond. Impacts leave craters, cause impact winters and mass extinctions.
* **Scenarios.** No Moon · 2× Jupiter · a brighter Sun · Chicxulub today · a rogue planet ·
  two moons · Mars with Earth's air.
