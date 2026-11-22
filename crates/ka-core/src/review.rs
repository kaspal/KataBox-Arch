//! Spaced review for quiz questions (M5.5, ADR 0010): "SM-2-lite".
//!
//! - A question answered correctly the first time never enters the queue.
//! - A miss puts it in the queue at step 0, due tomorrow.
//! - A correct review moves it up a step; the next review is `INTERVALS[step]` days later.
//!   A correct answer at the last step retires it.
//! - A wrong review sends it back to step 0 (due tomorrow) and counts a lapse.
//! - Due cards come most-lapsed first, then oldest due, then by id (the order key; it never
//!   changes an interval).
//!
//! Dates are local calendar days from the UI (`YYYY-MM-DD`), counted as days since
//! 1970-01-01 with no time zone and no clock: the schedule is pure and deterministic.

use serde::{Deserialize, Serialize};

/// Days until the next review, by step.
pub const INTERVALS: [i64; 4] = [1, 3, 7, 21];
/// The last step: a correct answer here retires the card.
pub const LAST_STEP: u8 = 3;

/// A calendar day: days since 1970-01-01.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day(pub i64);

impl Day {
    /// `YYYY-MM-DD` (a real date) → a day.
    pub fn parse(s: &str) -> Option<Day> {
        let mut parts = s.split('-');
        let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
            return None;
        }
        let (y, m, d): (i64, i64, i64) = (y.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
        if !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
            return None;
        }
        Some(Day(days_from_civil(y, m, d)))
    }

    pub fn plus(self, days: i64) -> Day {
        Day(self.0 + days)
    }

    /// The day as `YYYY-MM-DD`.
    pub fn iso(self) -> String {
        let (y, m, d) = civil_from_days(self.0);
        format!("{y:04}-{m:02}-{d:02}")
    }
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if is_leap(y) => 29,
        _ => 28,
    }
}

/// Howard Hinnant's `days_from_civil`: proleptic Gregorian date → days since 1970-01-01.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// A question in the review queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub step: u8,
    pub lapses: u32,
    pub due: Day,
    pub last: Day,
}

impl Serialize for Day {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.iso())
    }
}

impl<'de> Deserialize<'de> for Day {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Day, D::Error> {
        let s = String::deserialize(d)?;
        Day::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("bad day {s:?}")))
    }
}

/// What happens to a question's card after an answer on `today`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Not in the queue (answered correctly the first time, or retired).
    None,
    Card(Card),
}

/// The card after answering. `card` is the question's card, if it has one; `due` says
/// whether this answer is the scheduled review (an answer before the due day only matters
/// when it is wrong: a miss is always worth reviewing).
pub fn answer(card: Option<&Card>, correct: bool, today: Day) -> Outcome {
    let miss = |lapses: u32| {
        Outcome::Card(Card {
            step: 0,
            lapses,
            due: today.plus(INTERVALS[0]),
            last: today,
        })
    };
    match card {
        None if correct => Outcome::None,
        None => miss(0),
        Some(c) if !correct => miss(c.lapses + 1),
        Some(c) if c.due > today => Outcome::Card(c.clone()),
        Some(c) if c.step >= LAST_STEP => Outcome::None,
        Some(c) => {
            let step = c.step + 1;
            Outcome::Card(Card {
                step,
                lapses: c.lapses,
                due: today.plus(INTERVALS[usize::from(step)]),
                last: today,
            })
        }
    }
}

/// The due cards in review order: most lapses first, then the oldest due, then by id.
pub fn order<T>(cards: &mut [(String, Card, T)]) {
    cards.sort_by(|a, b| {
        b.1.lapses
            .cmp(&a.1.lapses)
            .then(a.1.due.cmp(&b.1.due))
            .then(a.0.cmp(&b.0))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(s: &str) -> Day {
        Day::parse(s).unwrap()
    }

    #[test]
    fn days_parse_and_print_across_month_and_year_edges() {
        assert_eq!(day("1970-01-01"), Day(0));
        assert_eq!(day("2026-11-22").iso(), "2026-11-22");
        assert_eq!(day("2026-12-31").plus(1).iso(), "2027-01-01");
        assert_eq!(day("2028-02-28").plus(1).iso(), "2028-02-29");
        assert_eq!(day("2027-02-28").plus(1).iso(), "2027-03-01");
        assert_eq!(
            day("2100-02-28").plus(1).iso(),
            "2100-03-01",
            "not a leap year"
        );
        assert_eq!(day("2000-02-28").plus(1).iso(), "2000-02-29", "a leap year");
        for bad in [
            "2026-02-30",
            "2026-13-01",
            "2026-1-01",
            "26-01-01",
            "2026-01-01-1",
            "x",
        ] {
            assert_eq!(Day::parse(bad), None, "{bad}");
        }
    }

    /// One question's life, answer by answer: (answer day, correct) → (due day or retired).
    #[test]
    fn the_schedule_follows_its_table() {
        let mut card: Option<Card> = None;
        let script = [
            ("2026-11-22", false, Some("2026-11-23")), // missed: due tomorrow
            ("2026-11-23", true, Some("2026-11-26")),  // step 1: +3
            ("2026-11-26", true, Some("2026-12-03")),  // step 2: +7
            ("2026-12-03", false, Some("2026-12-04")), // missed again: back to step 0
            ("2026-12-04", true, Some("2026-12-07")),
            ("2026-12-07", true, Some("2026-12-14")),
            ("2026-12-14", true, Some("2027-01-04")), // step 3: +21, across the year
            ("2027-01-04", true, None),               // correct at the last step: retired
        ];
        for (when, correct, want) in script {
            card = match answer(card.as_ref(), correct, day(when)) {
                Outcome::None => None,
                Outcome::Card(c) => Some(c),
            };
            assert_eq!(
                card.as_ref().map(|c| c.due.iso()),
                want.map(str::to_owned),
                "{when}"
            );
        }
    }

    #[test]
    fn first_answers_and_early_answers() {
        assert_eq!(answer(None, true, day("2026-11-22")), Outcome::None);
        let c = Card {
            step: 1,
            lapses: 2,
            due: day("2026-11-30"),
            last: day("2026-11-22"),
        };
        // Correct before it is due: nothing changes.
        assert_eq!(
            answer(Some(&c), true, day("2026-11-25")),
            Outcome::Card(c.clone())
        );
        // Wrong before it is due: back to step 0, one more lapse.
        match answer(Some(&c), false, day("2026-11-25")) {
            Outcome::Card(n) => assert_eq!(
                (n.step, n.lapses, n.due.iso().as_str()),
                (0, 3, "2026-11-26")
            ),
            Outcome::None => panic!("a miss keeps the card"),
        }
    }

    #[test]
    fn due_cards_come_most_lapsed_first_then_oldest() {
        let c = |lapses, due: &str| Card {
            step: 0,
            lapses,
            due: day(due),
            last: day(due),
        };
        let mut cards = vec![
            ("b".to_string(), c(0, "2026-11-20"), ()),
            ("a".to_string(), c(2, "2026-11-22"), ()),
            ("c".to_string(), c(0, "2026-11-19"), ()),
            ("d".to_string(), c(0, "2026-11-19"), ()),
        ];
        order(&mut cards);
        let ids: Vec<&str> = cards.iter().map(|x| x.0.as_str()).collect();
        assert_eq!(ids, ["a", "c", "d", "b"]);
    }
}
