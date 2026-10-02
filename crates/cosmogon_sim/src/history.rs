//! Meaningful events. The simulation never stores every tick — only what a historian would
//! write down, plus decimated per-civilization samples (see `civ::Samples`).

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    Astronomy,
    Life,
    Civilization,
    Technology,
    Disaster,
    War,
    Space,
    Contact,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Astronomy => "Astronomy",
            Category::Life => "Life",
            Category::Civilization => "Civilization",
            Category::Technology => "Technology",
            Category::Disaster => "Disaster",
            Category::War => "War",
            Category::Space => "Space",
            Category::Contact => "Contact",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Event {
    pub time: f64,
    pub category: Category,
    /// 1 (minor) ..= 5 (milestone).
    pub importance: u8,
    pub title: String,
    pub detail: String,
    pub system: Option<u32>,
    pub body: Option<u32>,
    pub civ: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct History {
    pub events: Vec<Event>,
}

impl History {
    /// Minor events beyond this count are pruned oldest-first; milestones are kept forever.
    pub const MINOR_CAP: usize = 4000;

    pub fn push(&mut self, e: Event) {
        self.events.push(e);
        let minor = self.events.iter().filter(|e| e.importance < 3).count();
        if minor > Self::MINOR_CAP {
            if let Some(i) = self.events.iter().position(|e| e.importance < 3) {
                self.events.remove(i);
            }
        }
    }

    pub fn for_civ(&self, civ: u32) -> impl Iterator<Item = &Event> {
        self.events.iter().filter(move |e| e.civ == Some(civ))
    }
}
