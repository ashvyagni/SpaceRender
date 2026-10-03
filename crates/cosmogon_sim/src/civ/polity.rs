//! Polities: the rival states within a civilization.
//!
//! A civilization (species-level society) shares knowledge, but its people are divided into
//! polities that own settlements. Territory is shaped by geography and reach (oceans divide
//! peoples until seafaring), and polities form, fight, conquer, unite, fracture and reform.
//! Updated whenever settlements are (every ~10 simulated years, longer in quiet eras), with
//! all probabilities scaled to the elapsed interval so step size never biases outcomes.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::settlements::{Adjacency, Site, Tier};
use super::CivEvent;
use crate::history::Category;
use crate::names;
use crate::rng::Rng;
use crate::time::group_digits;
use cosmogon_core::dmath::DMath;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Government {
    Chiefdom,
    Kingdom,
    Empire,
    Republic,
    Federation,
    WorldState,
    /// A real present-day country (title is its name).
    State,
}

impl Government {
    pub fn label(self) -> &'static str {
        match self {
            Government::Chiefdom => "Chiefdom",
            Government::Kingdom => "Kingdom",
            Government::Empire => "Empire",
            Government::Republic => "Republic",
            Government::Federation => "Federation",
            Government::WorldState => "World government",
            Government::State => "State",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Polity {
    pub id: u16,
    /// Base name; see [`Polity::title`].
    pub name: String,
    pub government: Government,
    /// Index into the civilization's sites.
    pub capital: u16,
    pub founded: f64,
    pub ended: Option<f64>,
    pub color: [u8; 3],
    pub population: f64,
    pub sites: u32,
    /// Opponents currently at war with this polity.
    pub at_war: Vec<u16>,
    /// Attitude towards other polities, −1 (hostile) … 1 (friendly).
    pub relations: BTreeMap<u16, f32>,
    pub wars_fought: u32,
}

impl Polity {
    pub fn alive(&self) -> bool {
        self.ended.is_none()
    }
    pub fn title(&self) -> String {
        match self.government {
            Government::Chiefdom => format!("{} chiefdom", self.name),
            Government::Kingdom => format!("Kingdom of {}", self.name),
            Government::Empire => format!("{} Empire", self.name),
            Government::Republic => format!("Republic of {}", self.name),
            Government::Federation => format!("{} Federation", self.name),
            Government::WorldState => format!("United {}", self.name),
            Government::State => self.name.clone(),
        }
    }
    fn relation(&self, other: u16) -> f32 {
        self.relations.get(&other).copied().unwrap_or(0.0)
    }
}

pub fn color_for(id: u16) -> [u8; 3] {
    // Golden-angle hues, fixed saturation/value: distinct and readable on dark terrain.
    let h = (id as f64 * 137.508).rem_euclid(360.0) / 60.0;
    let (s, v) = (0.62, 0.95);
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8]
}

fn over(p: f64, dt: f64) -> f64 {
    1.0 - (1.0 - p.clamp(0.0, 1.0)).dpowf(dt)
}

fn arc(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0).dacos()
}

/// Inputs from the civilization that drive politics.
pub struct PoliticsInput<'a> {
    pub flags: &'a std::collections::BTreeSet<String>,
    pub stability: f64,
    pub food_pressure: f64,
    pub reach: f64,
    pub seafaring: bool,
    pub knows_agriculture: bool,
}

/// Output: events and the fraction of the population killed in wars this interval.
pub struct PoliticsOutput {
    pub events: Vec<CivEvent>,
    pub war_deaths: f64,
    pub active_wars: u32,
}

/// Recount each polity's population and settlements from its territory.
pub fn tally(sites: &[Site], polities: &mut [Polity]) {
    for p in polities.iter_mut() {
        p.population = 0.0;
        p.sites = 0;
    }
    for s in sites.iter().filter(|s| s.active()) {
        if let Some(pid) = s.polity {
            if let Some(p) = polities.get_mut(pid as usize) {
                p.population += s.population;
                p.sites += 1;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update(sites: &mut [Site], polities: &mut Vec<Polity>, adj: &Adjacency, input: &PoliticsInput, dt: f64, t: f64, rng: &mut Rng) -> PoliticsOutput {
    let mut out = PoliticsOutput { events: Vec::new(), war_deaths: 0.0, active_wars: 0 };
    // Political change is resolved at least once a century; longer quiet intervals don't
    // compound into a burst of simultaneous wars.
    let dt = dt.min(100.0);
    if !input.knows_agriculture || adj.is_empty() {
        return out;
    }
    let ev = |out: &mut PoliticsOutput, importance: u8, category: Category, title: String, detail: String| out.events.push(CivEvent { importance, category, title, detail });
    let writing = input.flags.contains("writing");
    let reachable = |d: f32, native: bool| (d as f64) <= input.reach && (native || input.seafaring || d < 0.2);

    // 1. Unclaimed settlements join a reachable neighbour, or found a new polity.
    let mut order: Vec<usize> = (0..sites.len()).filter(|&i| sites[i].active() && sites[i].polity.is_none()).collect();
    order.sort_by(|&a, &b| sites[a].founded.unwrap_or(t).total_cmp(&sites[b].founded.unwrap_or(t)).then(a.cmp(&b)));
    for i in order {
        let neighbour = adj.near[i].iter().find(|&&(j, d, native)| reachable(d, native) && sites[j as usize].polity.is_some_and(|p| polities[p as usize].alive()));
        if let Some(&(j, _, _)) = neighbour {
            sites[i].polity = sites[j as usize].polity;
        } else if sites[i].tier() >= Tier::Village {
            let id = polities.len() as u16;
            let name = names::word(rng);
            let government = if writing { Government::Kingdom } else { Government::Chiefdom };
            polities.push(Polity { id, name, government, capital: i as u16, founded: t, ended: None, color: color_for(id), population: 0.0, sites: 0, at_war: Vec::new(), relations: BTreeMap::new(), wars_fought: 0 });
            sites[i].polity = Some(id);
            let p = &polities[id as usize];
            let first = polities.iter().filter(|q| q.alive()).count() == 1;
            ev(&mut out, if first { 4 } else { 2 }, Category::Civilization, format!("{} founded", p.title()), format!("A new polity arises around {}.", sites[i].name));
        }
    }

    // 2. Tally territory; polities without land are gone.
    for p in polities.iter_mut() {
        p.population = 0.0;
        p.sites = 0;
    }
    for s in sites.iter().filter(|s| s.active()) {
        if let Some(pid) = s.polity {
            let p = &mut polities[pid as usize];
            p.population += s.population;
            p.sites += 1;
        }
    }
    for i in 0..polities.len() {
        if polities[i].alive() && polities[i].sites == 0 {
            polities[i].ended = Some(t);
            let title = polities[i].title();
            ev(&mut out, 3, Category::War, format!("{title} falls"), "Its last settlement is lost.".into());
        }
    }
    let alive: Vec<u16> = polities.iter().filter(|p| p.alive()).map(|p| p.id).collect();
    for p in polities.iter_mut() {
        p.at_war.retain(|o| alive.contains(o));
    }

    // 3. Borders: pairs of polities whose settlements are within reach of each other.
    let mut borders: BTreeMap<(u16, u16), Vec<(usize, usize)>> = BTreeMap::new();
    for i in 0..sites.len() {
        let Some(a) = sites[i].polity.filter(|_| sites[i].active()) else { continue };
        for &(j, d, native) in &adj.near[i] {
            let j = j as usize;
            if !sites[j].active() || !reachable(d, native) {
                continue;
            }
            if let Some(b) = sites[j].polity {
                if a != b && alive.contains(&a) && alive.contains(&b) {
                    borders.entry((a.min(b), a.max(b))).or_default().push(if a < b { (i, j) } else { (j, i) });
                }
            }
        }
    }

    let strength = |p: &Polity| (p.population.max(1.0)).dpowf(0.6) * (0.5 + p.sites as f64 * 0.02);
    let connected = input.flags.contains("roads") as u32 as f64 + 2.0 * input.flags.contains("rail") as u32 as f64 + 4.0 * input.flags.contains("networks") as u32 as f64;

    for (&(a, b), front) in &borders {
        let (ai, bi) = (a as usize, b as usize);
        if !polities[ai].alive() || !polities[bi].alive() {
            continue;
        }
        let at_war = polities[ai].at_war.contains(&b);
        // Relations drift towards friendship in peace, worsen under hunger and instability.
        let drift = (0.004 * dt) as f32 - (0.01 * dt * input.food_pressure * (1.0 - input.stability)) as f32;
        for (x, y) in [(ai, b), (bi, a)] {
            let r = polities[x].relations.entry(y).or_insert(0.0);
            *r = (*r + drift).clamp(-1.0, 1.0);
        }
        let rel = polities[ai].relation(b) as f64;
        if !at_war {
            let war_rate = 0.0015 * (1.5 - input.stability) * (1.0 + 2.0 * input.food_pressure) * (1.0 - rel).max(0.1);
            // Peaceful unions are rare before modern communications made distant rule practical.
            let union_rate = 0.00003 * (1.0 + 3.0 * connected) * rel.max(0.0);
            if rng.chance(over(war_rate, dt)) {
                polities[ai].at_war.push(b);
                polities[bi].at_war.push(a);
                polities[ai].wars_fought += 1;
                polities[bi].wars_fought += 1;
                for (x, y) in [(ai, b), (bi, a)] {
                    polities[x].relations.insert(y, -0.8);
                }
                let (ta, tb) = (polities[ai].title(), polities[bi].title());
                let total: f64 = polities.iter().filter(|p| p.alive()).map(|p| p.population).sum::<f64>().max(1.0);
                let major = polities[ai].population / total > 0.1 && polities[bi].population / total > 0.1;
                ev(&mut out, if major { 3 } else { 2 }, Category::War, format!("War: {ta} against {tb}"), "Border tensions erupt into open war.".into());
            } else if rng.chance(over(union_rate, dt)) {
                // Peaceful union: the larger absorbs the smaller as a federation.
                let (big, small) = if polities[ai].population >= polities[bi].population { (ai, bi) } else { (bi, ai) };
                for s in sites.iter_mut() {
                    if s.polity == Some(small as u16) {
                        s.polity = Some(big as u16);
                    }
                }
                polities[small].ended = Some(t);
                let small_title = polities[small].title();
                polities[big].government = if input.flags.contains("printing") {
                    Government::Federation
                } else if !writing {
                    Government::Chiefdom
                } else if polities[big].sites > 25 {
                    Government::Empire
                } else {
                    Government::Kingdom
                };
                let big_title = polities[big].title();
                ev(&mut out, 4, Category::Civilization, format!("{small_title} joins the {big_title}"), "Two peoples unite peacefully under one government.".into());
            }
        } else {
            out.active_wars += 1;
            // Campaigns: the stronger side tends to take border settlements.
            let (sa, sb) = (strength(&polities[ai]), strength(&polities[bi]));
            let campaigns = (dt / 10.0).ceil().clamp(1.0, 5.0) as usize;
            for _ in 0..campaigns {
                let a_wins = rng.f64() < sa / (sa + sb);
                let (winner, loser) = if a_wins { (a, b) } else { (b, a) };
                if rng.chance(0.35) {
                    let candidates: Vec<usize> = front.iter().map(|&(x, y)| if sites[x].polity == Some(loser) { x } else { y }).filter(|&i| sites[i].polity == Some(loser)).collect();
                    if let Some(&target) = candidates.get(rng.range_u32(0, candidates.len().saturating_sub(1) as u32) as usize) {
                        sites[target].polity = Some(winner);
                        if sites[target].tier() >= Tier::City || target as u16 == polities[loser as usize].capital {
                            let (tw, tl) = (polities[winner as usize].title(), polities[loser as usize].title());
                            ev(&mut out, 3, Category::War, format!("{tw} captures {}", sites[target].name), format!("{} ({}) is taken from the {tl}.", sites[target].name, group_digits(sites[target].population)));
                        }
                    }
                }
            }
            out.war_deaths += 0.0015 * dt.min(50.0) * ((polities[ai].population + polities[bi].population) / sites.iter().map(|s| s.population).sum::<f64>().max(1.0));
            if rng.chance(over(0.06, dt)) {
                polities[ai].at_war.retain(|&x| x != b);
                polities[bi].at_war.retain(|&x| x != a);
                for (x, y) in [(ai, b), (bi, a)] {
                    polities[x].relations.insert(y, -0.3);
                }
                let (ta, tb) = (polities[ai].title(), polities[bi].title());
                ev(&mut out, 2, Category::War, format!("Peace between {ta} and {tb}"), "The war ends in a negotiated peace.".into());
            }
        }
    }

    // 4. Capitals: a polity that lost its capital moves it to its largest settlement.
    for p in polities.iter_mut().filter(|p| p.alive()) {
        if sites[p.capital as usize].polity != Some(p.id) {
            if let Some((i, _)) = sites.iter().enumerate().filter(|(_, s)| s.active() && s.polity == Some(p.id)).max_by(|a, b| a.1.population.total_cmp(&b.1.population)) {
                p.capital = i as u16;
            }
        }
    }

    // 5. Fracture: large states in unstable times split along distance from the capital.
    let ids: Vec<usize> = polities.iter().filter(|p| p.alive() && p.sites > 14).map(|p| p.id as usize).collect();
    for pi in ids {
        if !rng.chance(over(0.0015 * (1.0 - input.stability).max(0.0) * 2.0, dt)) {
            continue;
        }
        let cap = sites[polities[pi].capital as usize].dir();
        let members: Vec<usize> = (0..sites.len()).filter(|&i| sites[i].active() && sites[i].polity == Some(pi as u16)).collect();
        let Some(&far) = members.iter().max_by(|&&x, &&y| arc(cap, sites[x].dir()).total_cmp(&arc(cap, sites[y].dir()))) else { continue };
        let far_dir = sites[far].dir();
        let id = polities.len() as u16;
        let name = names::word(rng);
        let gov = if input.flags.contains("printing") { Government::Republic } else { Government::Kingdom };
        polities.push(Polity { id, name, government: gov, capital: far as u16, founded: t, ended: None, color: color_for(id), population: 0.0, sites: 0, at_war: vec![pi as u16], relations: BTreeMap::new(), wars_fought: 1 });
        polities[pi].at_war.push(id);
        let mut moved = 0;
        for &m in &members {
            if arc(far_dir, sites[m].dir()) < arc(cap, sites[m].dir()) {
                sites[m].polity = Some(id);
                moved += 1;
            }
        }
        let (tp, tn) = (polities[pi].title(), polities[id as usize].title());
        ev(&mut out, 3, Category::War, format!("Civil war: {tn} breaks away from the {tp}"), format!("{moved} settlements around {} secede.", sites[far].name));
    }

    // 6. Governments evolve with size and ideas.
    let total_pop: f64 = polities.iter().filter(|p| p.alive()).map(|p| p.population).sum::<f64>().max(1.0);
    let alive_count = polities.iter().filter(|p| p.alive()).count();
    for p in polities.iter_mut().filter(|p| p.alive()) {
        let before = p.government;
        p.government = match p.government {
            Government::Chiefdom if writing => Government::Kingdom,
            Government::Kingdom if p.sites > 25 => Government::Empire,
            Government::Kingdom | Government::Empire if input.flags.contains("printing") && rng.chance(over(0.002, dt)) => Government::Republic,
            g => g,
        };
        if alive_count == 1 && p.population / total_pop > 0.95 && input.flags.contains("networks") && p.government != Government::WorldState {
            p.government = Government::WorldState;
        }
        if p.government != before {
            let importance = if p.government == Government::WorldState { 5 } else { 2 };
            let detail = match p.government {
                Government::WorldState => "For the first time, the whole world is governed as one.".to_string(),
                Government::Republic => "A revolution replaces the monarchy with a republic.".to_string(),
                g => format!("The state becomes a {}.", g.label().to_lowercase()),
            };
            ev(&mut out, importance, Category::Civilization, format!("{} proclaimed", p.title()), detail);
        }
    }
    out
}
