# Cosmogon 1.0 — product plan

Written after 0.5.0, from research into the closest products and what players say about them.

## What the market tells us

| Product | Loved for | Criticised for |
|---|---|---|
| Universe Sandbox | Throwing planets, collisions, "what if" videos, VR | Unfriendly interface with dozens of unexplained buttons; slows down with many objects; stars that don't evolve when aged; life only now on its roadmap |
| SpaceEngine (96 % positive) | Seamless scale from a canyon to galaxy clusters; real catalogues + procedural universe | Little to *do*; no consequences, no life |
| Celestia | Free, accurate, educational | Dated look, static |
| Dwarf Fortress | Emergent stories; "Legends" lets you read every history | Hard to learn |
| Kerbal Space Program | Learning orbital mechanics by doing; goals | — |

**Cosmogon's position:** the only free simulator where *physics → climate → life →
civilization* is one causal chain. Other sandboxes answer "what happens to Earth's orbit?";
Cosmogon answers "what happens to the people?". 1.0 must (1) make that chain spectacular
and readable, (2) cover the universe's full cast of objects, (3) be easy and pleasant from
the first minute, and (4) produce stories worth sharing.

## Pillars and features (★ = in 1.0)

### A. The whole universe
* ★ **Life and death of stars**: red giants that swallow planets, planetary nebulae, white
  dwarfs, supernovae, neutron stars, black holes; consequences for every world (done in
  the simulation: engulfment, adiabatic orbit widening, vaporisation, unbinding, radiation
  fronts reaching other systems at light speed).
* ★ **Exotic objects to create and throw**: black holes (gravitational lensing, photon
  ring, accretion disk, spaghettification), neutron stars and pulsars (beams), white dwarfs,
  extra stars, rogue planets.
* ★ **Nebulae**: planetary nebulae, supernova remnants, emission nebulae around hot young
  stars, dark clouds; star-forming regions where new systems ignite.
* ★ **Galaxy scale**: the Milky Way seen from outside (arms, bulge, Sgr A*), real nearby
  stars from public catalogues, the Local Group, distant galaxies and quasars — one
  seamless zoom from a city street to the cosmic web.
* **Galactic collision sandbox** (Milky Way–Andromeda), gravitational waves from mergers,
  kilonovae, tidal disruption events.

### B. Spectacle physics
* ★ **Real collisions**: fragmentation and debris, molten aftermath glowing for years,
  moons torn into rings inside the Roche limit, atmospheric entry fireballs.
* ★ **Comets with tails** (ion and dust tails pointing away from the star), auroras on
  magnetised worlds.
* GPU N-body for thousands of particles (asteroid belts as real objects).

### C. Civilizations across the stars
* ★ **Interstellar colonisation**: generation ships to nearby systems found new branches of
  a civilization, which diverge; first contact between civilizations of different stars.
* ★ **Megastructures and terraforming** as technology endpoints: Dyson swarms (the
  Kardashev step to K≈2, visibly dimming the star), terraformed Mars greening over
  centuries, space elevators, orbital rings.
* ★ **The Chronicle as a book** ("Legends"): per-world and per-civilization histories with
  causes, exportable as a shareable story card.
* Fermi-paradox experiments: a galaxy map of who heard whom.

### D. Experience
* ★ **Redesigned interface**: icon set, clean cards, fewer and clearer controls, search-first
  command palette, consistent typography; tooltips that teach.
* ★ **Guided tours** (first run): "Our Solar System", "Life and death of the Sun",
  "Humanity's future", "Throw a black hole" — onboarding by doing.
* ★ **Timeline**: scrub through history with event markers; rewind to automatic
  checkpoints.
* ★ **Photo mode and video capture**; cinematic camera.
* ★ **Sound**: generated ambient score and sonification (pulsar clicks, impact rumbles,
  supernova roar) — procedurally synthesised, no licensing.
* **Challenges**: goals with outcomes ("Save Earth from the asteroid", "Get a civilization to
  the stars without nuclear war", "Make Mars habitable").
* **Observatory mode**: discover exoplanets yourself from transit light curves and
  radial-velocity wobbles.

### E. Product
* Signed/notarised installers, in-app update check, opt-in crash reports, a website,
  localisation, accessibility (UI scale, colour-blind palettes, reduced motion),
  mod folder for data files (technologies, species, presets).

## Order of work for 1.0
1. Stars: life and death + compact objects + black-hole and nebula rendering.
2. Galaxy layer + real nearby stars + Local Group + quasars.
3. Spectacle physics: fragmentation, Roche rings, comets.
4. Civilizations across the stars: interstellar colonies, contact, megastructures, terraforming.
5. Experience: UI redesign, tours, timeline, photo/video, sound, chronicle book.
6. Release 1.0.

Sources: Universe Sandbox Steam roadmap discussion and user reviews (Steam, Metacritic,
PC Gamer); SpaceEngine Steam reviews and press; Dwarf Fortress "Legends" (DF wiki).
