//! Simulation time.
//!
//! The universe clock is `f64` seconds relative to the J2000.0 epoch. At the extreme of
//! 10 billion years (3.2e17 s) the clock still resolves ~64 s, which is far finer than any
//! system that runs at those scales needs. Orbits are evaluated analytically at the clock
//! value; all stateful systems run on fixed periods through the [`crate::scheduler`].

pub const SECONDS_PER_DAY: f64 = 86_400.0;
/// Julian year.
pub const SECONDS_PER_YEAR: f64 = 365.25 * SECONDS_PER_DAY;
pub const SECONDS_PER_KYR: f64 = 1.0e3 * SECONDS_PER_YEAR;
pub const SECONDS_PER_MYR: f64 = 1.0e6 * SECONDS_PER_YEAR;
pub const SECONDS_PER_GYR: f64 = 1.0e9 * SECONDS_PER_YEAR;

pub fn years(t: f64) -> f64 {
    t / SECONDS_PER_YEAR
}

/// A selectable simulation speed (simulated seconds per real second).
#[derive(Clone, Copy, Debug)]
pub struct Speed {
    pub label: &'static str,
    pub rate: f64,
}

/// Speeds span from real time to geological time. Stateful systems are stepped on fixed
/// periods, so the fastest speeds are limited by the per-frame CPU budget, not by
/// accuracy: if the simulation can't keep up it falls behind rather than taking larger,
/// less accurate steps (see `scheduler`).
pub const SPEEDS: &[Speed] = &[
    Speed { label: "Real time", rate: 1.0 },
    Speed { label: "1 min/s", rate: 60.0 },
    Speed { label: "1 hr/s", rate: 3_600.0 },
    Speed { label: "1 day/s", rate: SECONDS_PER_DAY },
    Speed { label: "1 month/s", rate: SECONDS_PER_YEAR / 12.0 },
    Speed { label: "1 yr/s", rate: SECONDS_PER_YEAR },
    Speed { label: "10 yr/s", rate: 10.0 * SECONDS_PER_YEAR },
    Speed { label: "100 yr/s", rate: 100.0 * SECONDS_PER_YEAR },
    Speed { label: "1 kyr/s", rate: SECONDS_PER_KYR },
    Speed { label: "10 kyr/s", rate: 10.0 * SECONDS_PER_KYR },
    Speed { label: "100 kyr/s", rate: 100.0 * SECONDS_PER_KYR },
    Speed { label: "1 Myr/s", rate: SECONDS_PER_MYR },
    Speed { label: "10 Myr/s", rate: 10.0 * SECONDS_PER_MYR },
    Speed { label: "100 Myr/s", rate: 100.0 * SECONDS_PER_MYR },
];

/// Human-readable duration, choosing a sensible unit.
pub fn format_duration(seconds: f64) -> String {
    let s = seconds.abs();
    let sign = if seconds < 0.0 { "-" } else { "" };
    if s < 120.0 {
        format!("{sign}{s:.0} s")
    } else if s < 2.0 * 3600.0 {
        format!("{sign}{:.1} min", s / 60.0)
    } else if s < 2.0 * SECONDS_PER_DAY {
        format!("{sign}{:.1} h", s / 3600.0)
    } else if s < SECONDS_PER_YEAR {
        format!("{sign}{:.1} d", s / SECONDS_PER_DAY)
    } else if s < 1e4 * SECONDS_PER_YEAR {
        format!("{sign}{:.1} yr", s / SECONDS_PER_YEAR)
    } else if s < 1e6 * SECONDS_PER_YEAR {
        format!("{sign}{:.1} kyr", s / SECONDS_PER_KYR)
    } else if s < 1e9 * SECONDS_PER_YEAR {
        format!("{sign}{:.2} Myr", s / SECONDS_PER_MYR)
    } else {
        format!("{sign}{:.2} Gyr", s / SECONDS_PER_GYR)
    }
}

/// Group digits of a large number: 1234567 -> "1,234,567".
pub fn group_digits(v: f64) -> String {
    let neg = v < 0.0;
    let s = format!("{:.0}", v.abs());
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if neg {
        format!("-{out}")
    } else {
        out
    }
}

/// Calendar label. For the Sol scenario the Gregorian year is meaningful; elsewhere we
/// show elapsed time since the universe was created.
pub fn format_date(t: f64, start: f64, gregorian: bool) -> String {
    if gregorian {
        let year = 2000.0 + years(t);
        if year >= 1.0 {
            let y = year.floor();
            let doy = ((year - y) * 365.25).floor() + 1.0;
            format!("{} CE, day {:.0}", year_digits(y), doy)
        } else {
            format!("{} BCE", year_digits((1.0 - year).floor()))
        }
    } else {
        format!("Year {}", group_digits(years(t - start).floor()))
    }
}

/// Years are written without separators below 10 000 (2026, not 2,026).
fn year_digits(y: f64) -> String {
    if y.abs() < 10_000.0 {
        format!("{y:.0}")
    } else {
        group_digits(y)
    }
}

/// Proleptic Gregorian calendar date of `t` (TDB seconds since J2000.0):
/// (year — astronomical numbering, 0 = 1 BCE — month 1–12, day 1–31, seconds of day).
pub fn civil_date(t: f64) -> (i64, u32, u32, f64) {
    let jd = 2_451_545.0 + t / SECONDS_PER_DAY;
    let z = (jd + 0.5).floor();
    let sod = (jd + 0.5 - z) * SECONDS_PER_DAY;
    let z = z as i64;
    // Richards (2013), "Calendars", Explanatory Supplement to the Astronomical Almanac.
    let a = z + 32_044;
    let b = (4 * a + 3).div_euclid(146_097);
    let c = a - (146_097 * b).div_euclid(4);
    let d = (4 * c + 3).div_euclid(1461);
    let e = c - (1461 * d).div_euclid(4);
    let m = (5 * e + 2).div_euclid(153);
    let day = e - (153 * m + 2).div_euclid(5) + 1;
    let month = m + 3 - 12 * (m / 10);
    let year = 100 * b + d - 4800 + m / 10;
    (year, month as u32, day as u32, sod)
}

/// "1 Jan 2026 · 14:32" — a full date and time for sandbox clocks.
pub fn format_datetime(t: f64) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (y, m, d, sod) = civil_date(t);
    let (h, min) = ((sod / 3600.0).floor() as u32, ((sod % 3600.0) / 60.0).floor() as u32);
    let year = if y >= 1 { year_digits(y as f64) } else { format!("{} BCE", year_digits((1 - y) as f64)) };
    format!("{d} {} {year} · {h:02}:{min:02}", MONTHS[(m - 1) as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_units() {
        assert_eq!(format_duration(30.0), "30 s");
        assert!(format_duration(3.0 * SECONDS_PER_YEAR).ends_with("yr"));
        assert!(format_duration(5.0 * SECONDS_PER_MYR).ends_with("Myr"));
    }

    #[test]
    fn digit_grouping() {
        assert_eq!(group_digits(1234567.0), "1,234,567");
        assert_eq!(group_digits(999.0), "999");
        assert_eq!(group_digits(-1000.0), "-1,000");
    }

    #[test]
    fn gregorian_dates() {
        assert!(format_date(0.0, 0.0, true).starts_with("2000 CE"));
        assert_eq!(civil_date(0.0), (2000, 1, 1, 43_200.0));
        // 2026-01-01 00:00 TDB = JD 2461041.5
        let t = (2_461_041.5 - 2_451_545.0) * SECONDS_PER_DAY;
        assert_eq!(format_datetime(t), "1 Jan 2026 · 00:00");
        assert_eq!(civil_date(t + 59.0 * SECONDS_PER_DAY).1, 3); // 1 March (2026 is not a leap year)
        assert!(format_date(-200_000.0 * SECONDS_PER_YEAR, 0.0, true).ends_with("BCE"));
    }
}
