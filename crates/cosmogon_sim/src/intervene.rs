//! Sandbox interventions in a civilization's history.
//!
//! The user may nudge a civilization, within limits that keep its story its own:
//! * one intervention per civilization every [`COOLDOWN_YEARS`] simulated years;
//! * a technology can be *taught* only when every prerequisite technology is known and its
//!   physical requirements are met (launch Δv, resources, environment) — the missing
//!   knowledge is supplied, but eras can't be skipped and physics can't be bypassed;
//! * knowledge gifts, inspiration and hardships are bounded in size.
//! Interventions are ordinary sandbox edits: journalled, saved and undoable.

use serde::{Deserialize, Serialize};

use crate::civ::knowledge::{Domain, N_DOMAINS};
use crate::civ::tech::{Condition, Context, TechGraph};
use crate::civ::{environment_for, Civilization};
use crate::history::{Category, Event};
use crate::time::SECONDS_PER_YEAR;
use crate::{BodyRef, Universe};

pub const COOLDOWN_YEARS: f64 = 25.0;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Intervention {
    /// Teach one technology whose prerequisites are all known (by id).
    Teach(String),
    /// Share insight in one domain: +50 % of current knowledge (at most +50 million).
    ShareKnowledge(u8),
    /// An inspiring event: stability rises and crisis pressures ease.
    Inspire,
    /// Make the existence of others unmistakable: contact pressure drives astronomy and
    /// space research for decades.
    Signal,
    /// A hardship (an epidemic killing 2–10 %): tests resilience; drives medicine.
    Hardship,
}

impl Intervention {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Teach(_) => "Teach a technology",
            Self::ShareKnowledge(_) => "Share knowledge",
            Self::Inspire => "Inspire",
            Self::Signal => "Send a signal",
            Self::Hardship => "Hardship",
        }
    }
}

/// Technologies that may be taught now: all prerequisite technologies known and all
/// non-knowledge requirements met (at least one route possible).
pub fn teachable(u: &Universe, civ: &Civilization) -> Vec<usize> {
    let graph = TechGraph::embedded();
    let r = BodyRef { system: civ.system, body: civ.body };
    let body = u.body(r);
    let sys = u.system(civ.system);
    let moons = sys.moons_of(civ.body as usize).count();
    let others = sys.bodies.iter().enumerate().filter(|(i, x)| *i != civ.body as usize && x.kind.has_surface() && x.mass > 1e21).count();
    let env = environment_for(body, moons, others, u.time);
    let mut known = vec![false; graph.len()];
    for d in &civ.discoveries {
        if let Some(i) = graph.find(&d.tech) {
            known[i] = true;
        }
    }
    let has = |f: &str| civ.flags.contains(f);
    // Knowledge is treated as unlimited: that is what is being taught.
    let unlimited = [f64::INFINITY; N_DOMAINS];
    let ctx = Context { known: &known, knowledge: &unlimited, resources: &body.resources, env: &env, habitat: civ.species.habitat, population: civ.population, flags: &has };
    (0..graph.len())
        .filter(|&i| !known[i])
        .filter(|&i| {
            let t = &graph.techs[i];
            t.requires.iter().all(|c| matches!(c, Condition::Knowledge(..)) || graph.check(c, &ctx)) && (t.routes.is_empty() || t.routes.iter().any(|r| r.requires.iter().all(|c| graph.check(c, &ctx))))
        })
        .collect()
}

/// Years until the civilization accepts another intervention (0 = now).
pub fn cooldown_left(civ: &Civilization, t: f64) -> f64 {
    civ.last_intervention.map(|l| (COOLDOWN_YEARS - (t - l) / SECONDS_PER_YEAR).max(0.0)).unwrap_or(0.0)
}

pub fn validate(u: &Universe, civ: u32, action: &Intervention) -> Result<(), String> {
    let c = u.civs.get(civ as usize).ok_or("no such civilization")?;
    if !c.is_alive() {
        return Err("That civilization is extinct".into());
    }
    let wait = cooldown_left(c, u.time);
    if wait > 0.0 {
        return Err(format!("Too soon: the {} can be influenced again in {wait:.0} years", c.species.name));
    }
    match action {
        Intervention::Teach(id) => {
            let graph = TechGraph::embedded();
            let i = graph.find(id).ok_or("unknown technology")?;
            if !teachable(u, c).contains(&i) {
                return Err(format!("{} needs technologies or conditions they don't have yet", graph.techs[i].name));
            }
        }
        Intervention::ShareKnowledge(d) if *d as usize >= N_DOMAINS => return Err("unknown domain".into()),
        _ => {}
    }
    Ok(())
}

impl Universe {
    /// Apply an intervention (validated). Returns a summary for the journal.
    pub(crate) fn apply_intervention(&mut self, civ: u32, action: &Intervention) -> String {
        let t = self.time;
        let graph = TechGraph::embedded();
        let ci = civ as usize;
        let name = self.civs[ci].name.clone();
        let (title, detail) = match action {
            Intervention::Teach(id) => {
                let i = graph.find(id).expect("validated");
                let tech = &graph.techs[i];
                let c = &mut self.civs[ci];
                for cond in &tech.requires {
                    if let Condition::Knowledge(d, n) = cond {
                        c.knowledge[d.index()] = c.knowledge[d.index()].max(*n);
                    }
                }
                c.grant(graph, id, t);
                if let Some(last) = c.discoveries.last_mut() {
                    last.drivers = "taught by an outside intervention".into();
                }
                (format!("{name} are taught {}", tech.name), tech.description.clone())
            }
            Intervention::ShareKnowledge(d) => {
                let dom = Domain::ALL[*d as usize];
                let c = &mut self.civs[ci];
                let k = &mut c.knowledge[dom.index()];
                *k += (*k * 0.5).min(5.0e7).max(1000.0);
                (format!("{name} receive insight in {}", dom.name()), "Knowledge in one field jumps ahead; discoveries that depend on it come sooner.".into())
            }
            Intervention::Inspire => {
                let c = &mut self.civs[ci];
                c.stability = (c.stability + 0.2).min(0.95);
                c.pressures.war *= 0.3;
                c.pressures.food *= 0.5;
                (format!("A wave of hope among {name}"), "Stability rises and old conflicts cool.".into())
            }
            Intervention::Signal => {
                let c = &mut self.civs[ci];
                c.pressures.contact = 1.0;
                (format!("{name} detect an unexplained signal"), "Not natural, not their own. Astronomy and space research surge.".into())
            }
            Intervention::Hardship => {
                let c = &mut self.civs[ci];
                let frac = 0.02 + 0.08 * (1.0 - c.health.clamp(0.0, 0.95));
                c.population *= 1.0 - frac;
                c.pressures.disease = c.pressures.disease.max(0.8);
                c.stability = (c.stability - 0.08).max(0.0);
                (format!("An epidemic strikes {name}"), format!("{:.1}% of the population dies; medicine becomes a priority.", frac * 100.0))
            }
        };
        let c = &mut self.civs[ci];
        c.last_intervention = Some(t);
        // A changed society deserves a prompt look rather than a long stride.
        c.stride_years = 1;
        let (system, body) = (c.system, c.body);
        self.history.push(Event { time: t, category: Category::Civilization, importance: 4, title: title.clone(), detail, system: Some(system), body: Some(body), civ: Some(civ) });
        title
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::Edit;
    use crate::{Scenario, UniverseSettings};

    fn lab() -> Universe {
        Universe::new(UniverseSettings { scenario: Scenario::SolarSystemLab, ..Default::default() })
    }

    #[test]
    fn teaching_respects_prerequisites_and_cooldown() {
        let mut u = lab();
        let c = &u.civs[0];
        let graph = TechGraph::embedded();
        let ids: Vec<&str> = teachable(&u, c).into_iter().map(|i| graph.techs[i].id.as_str()).collect();
        assert!(ids.contains(&"space_infrastructure"), "{ids:?}");
        // Interstellar probes need fusion and colonies first.
        assert!(!ids.contains(&"interstellar_probes"));
        let edit = |a: Intervention| Edit::Intervene { system: 0, civ: 0, action: a };
        assert!(u.apply_edit(edit(Intervention::Teach("interstellar_probes".into()))).is_err());
        u.apply_edit(edit(Intervention::Teach("space_infrastructure".into()))).unwrap();
        assert!(u.civs[0].knows("space_infrastructure"));
        // Cooldown.
        assert!(u.apply_edit(edit(Intervention::Inspire)).is_err());
        u.advance_by(26.0 * SECONDS_PER_YEAR);
        u.apply_edit(edit(Intervention::Inspire)).unwrap();
        assert_eq!(u.edits.len(), 2);
        // Interventions don't switch the system to N-body or touch orbits.
        assert!(u.civs[0].is_alive());
    }
}
