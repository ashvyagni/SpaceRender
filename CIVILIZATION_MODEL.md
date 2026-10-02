# Civilization model

A civilization is a **statistical society** stepped once per simulated year
([`crates/cosmogon_sim/src/civ/mod.rs`](crates/cosmogon_sim/src/civ/mod.rs)). Everything it does is caused by
inspectable state; the inspector's *Civilization* tab shows it.

## Species
Generated from the home world: aquatic or land-dwelling (water worlds favour aquatic), body mass from gravity,
lifespan, sociality (affects research and stability), and descriptive traits. **Aquatic species cannot use fire**,
which blocks the metallurgical path — an outcome, not a rule about "who wins".

## Yearly step
1. **Capacity** = habitable area (land, or shallow seas for aquatic species) × fertile fraction × density from
   technology × climate (glacial swings −30%, warming stress, energy shortages).
2. **Population**: logistic growth (base 0.04%/yr, raised by agriculture and medicine). Overshoot → famine and food pressure.
3. **Research** = `scale · P^0.75 · research multipliers · sociality · stability`, distributed over 13 knowledge
   domains by *focus*: environment (oceans → navigation, moons and clear skies → astronomy, cold → energy) ×
   technology boosts × **pressures** (food → agriculture, disease → medicine/biology, energy shortage → energy,
   war → materials/engineering, warming → physics/chemistry, contact → astronomy).
   Knowledge decays: fast without writing (oral tradition), slowly with writing, almost not at all with printing.
4. **Discovery** — see [TECHNOLOGY_MODEL.md](TECHNOLOGY_MODEL.md).
5. **Energy**: demand = population × W/person; the fossil share is limited by remaining coal/oil; fossil burning
   depletes reserves and adds CO₂ (which warms the planet through the climate model and raises climate pressure).
6. **Shocks**: pandemics (urbanisation, domesticated animals, low health), wars (instability, hunger; nuclear war
   possible with nuclear weapons and very low stability), natural disasters (geology).
7. **Stability** drifts toward a technology-dependent baseline minus hunger, climate stress, energy shortage, war.
   Below the threshold → **collapse**: population halves, knowledge is lost (less if printed), a dark age until recovery.
8. **Settlements** (every 10 years): see below.
9. **Space**: satellites grow with energy use; colonies on suitable bodies of the home system; interstellar probes
   at 5% of c; radio spheres expand at c and can be detected by civilizations with radio astronomy.
10. Extinction below 500 individuals; the biosphere can later produce a new intelligent species.

## Settlements and networks
Each civilization owns ≤ 260 candidate sites chosen from the authoritative terrain (fertility, coast, climate,
nearby deposits). Expansion occupies the best unoccupied site **within reach** of an occupied one (walking ≈
0.11 rad per decade; boats allow short water gaps; ships allow ocean crossings; roads/rail/aircraft extend reach).
Urban population follows a Zipf-like rule by founding order (older = bigger) with a steeper slope as urbanisation
rises; rural population is spread across sites up to a band/village cap. Tiers: camp → village → town → city →
metropolis → megacity. Roads/rail link nearby towns over land; sea lanes link coastal towns across water.

## Calibration
The Sol scenario (humans with fire and stone tools, 200,000 years ago) is the reference trajectory. Across seeds:
agriculture 9,650–7,960 BCE (real ≈ 9,500 BCE, *caused* by the end of the last glacial period), writing
4,760–2,240 BCE (real ≈ 3,200), steam power 690 BCE–1670 CE (real 1712), radio 90–2608 CE (real 1895).
Real history lies inside the distribution of outcomes; nothing is scripted.

## Simplifications to remove later (see ROADMAP)
One polity per planet; no internal economy beyond energy and food; no cultures or languages yet; colonies are
records rather than full societies; mature civilizations are still stepped yearly (CPU-bound at high speeds).
