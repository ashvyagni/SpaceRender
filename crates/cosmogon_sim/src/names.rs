//! Deterministic procedural names.

use crate::rng::Rng;

const ONSETS: &[&str] = &[
    "", "b", "c", "d", "f", "g", "h", "k", "l", "m", "n", "p", "r", "s", "t", "v", "z", "th", "sh",
    "kr", "tr", "dr", "vel", "ax", "qu", "br", "gl", "st",
];
const VOWELS: &[&str] = &["a", "e", "i", "o", "u", "ae", "ei", "ou", "ia", "y"];
const CODAS: &[&str] = &["", "", "", "n", "r", "s", "l", "th", "x", "m", "nd", "rk", "ss"];

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// A pronounceable name of 2–3 syllables.
pub fn word(rng: &mut Rng) -> String {
    let syllables = rng.range_u32(2, 3);
    let mut s = String::new();
    for i in 0..syllables {
        s.push_str(ONSETS[rng.range_u32(0, ONSETS.len() as u32 - 1) as usize]);
        s.push_str(VOWELS[rng.range_u32(0, VOWELS.len() as u32 - 1) as usize]);
        if i + 1 == syllables || rng.chance(0.25) {
            s.push_str(CODAS[rng.range_u32(0, CODAS.len() as u32 - 1) as usize]);
        }
    }
    capitalize(&s)
}

/// Exoplanet-style letter suffix for the n-th planet (0 -> "b").
pub fn planet_letter(n: usize) -> char {
    (b'b' + (n as u8).min(24)) as char
}

pub fn roman(n: usize) -> &'static str {
    const R: [&str; 12] = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI", "XII"];
    R.get(n).copied().unwrap_or("XII+")
}
