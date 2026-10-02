//! Knowledge domains.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Domain {
    Mathematics,
    Astronomy,
    Physics,
    Chemistry,
    Biology,
    Medicine,
    Materials,
    Engineering,
    Agriculture,
    Navigation,
    Computing,
    Energy,
    Communications,
}

pub const N_DOMAINS: usize = 13;

impl Domain {
    pub const ALL: [Domain; N_DOMAINS] = [
        Domain::Mathematics,
        Domain::Astronomy,
        Domain::Physics,
        Domain::Chemistry,
        Domain::Biology,
        Domain::Medicine,
        Domain::Materials,
        Domain::Engineering,
        Domain::Agriculture,
        Domain::Navigation,
        Domain::Computing,
        Domain::Energy,
        Domain::Communications,
    ];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn name(self) -> &'static str {
        match self {
            Domain::Mathematics => "Mathematics",
            Domain::Astronomy => "Astronomy",
            Domain::Physics => "Physics",
            Domain::Chemistry => "Chemistry",
            Domain::Biology => "Biology",
            Domain::Medicine => "Medicine",
            Domain::Materials => "Materials",
            Domain::Engineering => "Engineering",
            Domain::Agriculture => "Agriculture",
            Domain::Navigation => "Navigation",
            Domain::Computing => "Computing",
            Domain::Energy => "Energy",
            Domain::Communications => "Communications",
        }
    }
    pub fn parse(s: &str) -> Option<Domain> {
        Domain::ALL.into_iter().find(|d| d.name().eq_ignore_ascii_case(s))
    }
}

/// Accumulated knowledge per domain (arbitrary "insight" units; see TECHNOLOGY_MODEL.md).
pub type Knowledge = [f64; N_DOMAINS];
