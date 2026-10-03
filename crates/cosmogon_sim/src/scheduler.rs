//! Multi-rate fixed-period scheduler.
//!
//! Each task runs on its own fixed period (biospheres every 10 kyr, civilizations every
//! year…). Due times are computed as `origin + k·period` from an integer step count, so
//! there is no accumulated floating-point drift, and tasks due at the same instant run in a
//! fixed order. The result: **the simulation's outcome does not depend on frame rate,
//! simulation speed, or how real time is chopped into frames.**

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Task {
    pub name: String,
    pub origin: f64,
    pub period: f64,
    pub steps: u64,
}

impl Task {
    pub fn next_due(&self) -> f64 {
        self.origin + (self.steps + 1) as f64 * self.period
    }
}

/// Period of a placeholder task that never runs (finite so saves stay valid JSON).
pub const NEVER: f64 = 1.0e290;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Scheduler {
    pub tasks: Vec<Task>,
}

impl Scheduler {
    pub fn add(&mut self, name: &str, origin: f64, period: f64) -> usize {
        self.tasks.push(Task { name: name.into(), origin, period, steps: 0 });
        self.tasks.len() - 1
    }

    /// The earliest due task, ties broken by registration order.
    pub fn next(&self) -> Option<(usize, f64)> {
        self.tasks
            .iter()
            .enumerate()
            .map(|(i, t)| (i, t.next_due()))
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
    }

    pub fn complete(&mut self, task: usize) {
        self.tasks[task].steps += 1;
    }

    /// Skip a task forward so its next run is step index `k` (never backwards). Used when a
    /// task knows nothing will happen for a while; the decision must depend only on
    /// simulation state so determinism is preserved.
    pub fn skip_to(&mut self, task: usize, k: u64) {
        let t = &mut self.tasks[task];
        t.steps = t.steps.max(k);
    }

    /// The first step index of `task` whose due time is at or after `time`.
    pub fn index_at_or_after(&self, task: usize, time: f64) -> u64 {
        let t = &self.tasks[task];
        (((time - t.origin) / t.period).ceil() - 1.0).max(0.0) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(chunks: &[f64]) -> Vec<(usize, u64)> {
        let mut s = Scheduler::default();
        s.add("slow", 0.0, 10.0);
        s.add("fast", 0.0, 1.0);
        let mut log = Vec::new();
        let mut now = 0.0;
        for c in chunks {
            now += c;
            while let Some((i, due)) = s.next() {
                if due > now {
                    break;
                }
                log.push((i, s.tasks[i].steps));
                s.complete(i);
            }
        }
        log
    }

    #[test]
    fn order_is_independent_of_frame_chunking() {
        let a = run(&[100.0]);
        let b = run(&[0.3; 334]);
        let c = run(&[7.0, 13.0, 0.5, 79.5]);
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a.iter().filter(|e| e.0 == 0).count(), 10);
        assert_eq!(a.iter().filter(|e| e.0 == 1).count(), 100);
    }
}
