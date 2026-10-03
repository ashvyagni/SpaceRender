# Data sources

Every external dataset used by Cosmogon, how it is obtained, and how to credit it.
Datasets are fetched by scripts in `tools/`, normalised, validated and cached in the
repository (or downloaded into the user's data directory for large catalogues). The
application never queries remote services while simulating.

## In use

### NASA/JPL Horizons — Solar System state vectors
| | |
|---|---|
| Provider | NASA Jet Propulsion Laboratory, Solar System Dynamics group |
| Dataset | Horizons On-Line Ephemeris System (planetary ephemeris DE441; satellite solutions jup365, sat441l, ura184, nep098, mar099) |
| License | US Government work, not subject to US copyright; attribution requested |
| Retrieval | `python3 tools/fetch_horizons.py [YYYY-MM-DD]` (Horizons REST API, VECTORS, centre `500@0`) |
| Cached as | `crates/cosmogon_sim/data/horizons_sol.toml` (dataset version 1, epoch 2026-01-01 00:00 TDB) |
| Fields | barycentric position (km) and velocity (km/s), ecliptic J2000 frame, TDB |
| Uncertainty | ephemeris uncertainty ≪ 1 km for planets; cached to 10 significant digits (≤ 100 m at 5 AU) |
| Update | re-run the script, bump `DATASET_VERSION`; existing sandboxes keep their copied initial state |
| Attribution | "Solar System data: NASA/JPL Horizons On-Line Ephemeris System" |

### Solar System physical parameters
| | |
|---|---|
| Provider | NASA Goddard Planetary Fact Sheets; IAU WG on Cartographic Coordinates and Rotational Elements |
| Dataset | masses, radii, rotation, obliquity, albedo, atmospheres, mean temperatures |
| License | public domain (NASA) |
| Retrieval | hand-entered (15 bodies) in `crates/cosmogon_sim/data/sol.toml`, marked MEASURED |
| Update | manual; to be replaced by Horizons physical data / JPL SSD tables (milestone S4) |

### NOAA ETOPO5 — Earth relief
| | |
|---|---|
| Provider | NOAA National Centers for Environmental Information |
| License | public domain |
| Retrieval | `tools/process_etopo.py` (see [ASSETS.md](../ASSETS.md)) |
| Cached as | `crates/cosmogon_sim/data/earth_elevation.bin`, 2048×1024 int16 metres |

### Constants
IAU 2012 astronomical unit (exact), IAU 2015 nominal solar values (GM☉ = 1.3271244 × 10²⁰
m³ s⁻², R☉ = 6.957 × 10⁸ m, L☉ = 3.828 × 10²⁶ W), CODATA 2018 G = 6.674 30 × 10⁻¹¹.

## Planned (evaluated, not yet ingested)

| Source | Use | License / limits | Plan |
|---|---|---|---|
| ESA Gaia DR3 (gaia_source, astrophysical_parameters) | positions, parallax, proper motion, RV, photometry, Teff | CC BY-SA 3.0 IGO; 1.8 × 10⁹ rows | magnitude-limited extracts by HEALPix tile via the ESA Gaia archive (TAP/ADQL); streamed tiles in the user data dir (S4/S5) |
| HYG / AT-HYG | bright-star convenience subset | CC BY-SA 4.0 | bootstrap the nearby-star layer (S5) |
| NASA Exoplanet Archive (PSCompPars, TAP) | planets, hosts, orbital & physical parameters with uncertainties | public, attribution requested | offline extract with uncertainty columns preserved (S5) |
| JPL Small-Body Database (SBDB API) | asteroids, comets, orbital elements, physical data | public | filtered extracts (e.g. H < 15 and all named comets) (S3/S4) |
| Minor Planet Center (MPCORB) | complete asteroid orbits | free to use with attribution | optional large download (S4) |
| CDS SIMBAD / VizieR | galaxies, clusters, nebulae, pulsars (ATNF), binaries, variables, white dwarfs | per-catalogue licences, usage policies and rate limits | one catalogue at a time, each with its own entry here (S5/S6) |

For each new source this file must record: provider, dataset, license, retrieval method,
update method, fields imported, uncertainty handling and attribution requirements.
