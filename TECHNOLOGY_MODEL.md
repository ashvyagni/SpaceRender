# Technology model

The technology graph is data: [`crates/cosmogon_sim/data/technologies.toml`](crates/cosmogon_sim/data/technologies.toml)
(56 technologies, Paleolithic → interstellar probes). It is **a graph of causal prerequisites, not a tree with timers**.

## Discoverability
A technology is discoverable when **all** `requires` conditions hold and, if it has `route`s, **at least one**
route's conditions hold. Conditions:

| Syntax | Meaning |
|---|---|
| `tech:ID` | another technology is known |
| `k:Domain>=N` | accumulated knowledge in one of 13 domains |
| `res:KEY>=X` | planetary resource endowment (Earth = 1): iron, copper, tin, coal, oil, uranium, rare_metals, fertile_land, fresh_water |
| `env:KEY>=X` / `<=X` | oxygen, gravity, ocean, land, pressure, temperature, launch_dv, moons, other_bodies, climate_stability |
| `habitat:land` / `water` | the species' native medium |
| `pop>=N` | population |
| `flag:NAME` | a capability unlocked elsewhere |

## Discovery
Once discoverable, a technology is found by a Poisson process with rate

```
speed(route) × surplus^1.5 × (1 + 2 × pressure(demand)) × tech_rate / years
```

where *surplus* is how far knowledge exceeds the minimum (capped at 4×) and *demand* names a societal pressure
(food, disease, energy, war, climate, contact, cold). Different civilizations therefore discover things in
different orders, sometimes by different routes, and some never at all. Every discovery records its **route and
drivers** ("knowledge at 1.8× the minimum; spurred by disease pressure (70%); route: coal-fired"), and the inspector
lists the near frontier with exactly what is missing.

## Examples of causality built in
* **No tin → no Bronze Age.** Iron working stays reachable directly from copper metallurgy (slower), or even from
  kilns alone (much slower).
* **No coal → steam power only from wood/peat** at 20% speed; **no oil → no internal combustion**, which in turn gates
  flight and rocketry.
* **No free oxygen or an aquatic species → no fire**, so no smelting, no pottery kilns, no industry.
* **Agriculture needs a stable (interglacial) climate**, which is why it appears after the last ice age on Earth.
* **Writing needs a large agricultural state** (bookkeeping), not just time.
* **Uranium-poor (old) worlds** struggle to reach fission; fusion depends on fission research.
* **Massive worlds** whose Δv to orbit exceeds ~12.5 km/s cannot reach orbit with chemical rockets; nuclear-thermal
  rockets are the only route, up to ~20 km/s; beyond that, never (Hippke 2018's "super-Earth trap").
* **A large moon** speeds calendrical astronomy and is required for a moon landing.
* Epidemics raise disease pressure, which accelerates medicine.

## Validation
On load the graph is checked for duplicate ids, unknown references and dependency cycles (unit tests cover
all three, plus the tin, aquatic-fire and heavy-world cases above).

## Editing
Add a `[[tech]]` block; reference only existing ids; keep `years` as the mean discovery time *at the minimum
knowledge*; prefer adding routes over adding special cases in code. Run `cargo test -p cosmogon_sim` and
`cosmogon-cli run --scenario sol --seed N` for several seeds to see the effect on history.
