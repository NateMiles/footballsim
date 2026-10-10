//! Competition formats and the match calendar.

use fs_core::GameDate;
use fs_core::date::Weekday;

/// First possible league matchday of the season starting in `year`.
pub fn season_opening(year: i32) -> GameDate {
    let mut d = GameDate::from_ymd(year, 8, 8).expect("valid date");
    while d.weekday() != Weekday::Saturday {
        d = d.add_days(1);
    }
    d
}

/// Last possible league matchday of the season starting in `year`.
pub fn season_closing(year: i32) -> GameDate {
    GameDate::from_ymd(year + 1, 5, 24).expect("valid date")
}

/// Dates for `rounds` league rounds in the season starting in `year`,
/// spread evenly over the Saturdays between the opening and closing
/// dates. Midweek (Wednesday) rounds are added only when there are more
/// rounds than Saturdays.
pub fn season_dates(year: i32, rounds: usize) -> Vec<GameDate> {
    if rounds == 0 {
        return Vec::new();
    }
    let (open, close) = (season_opening(year), season_closing(year));
    let saturdays: Vec<GameDate> = (0..)
        .map(|w| open.add_days(7 * w))
        .take_while(|d| *d <= close)
        .collect();
    let pool = if rounds <= saturdays.len() {
        saturdays
    } else {
        let mut both: Vec<GameDate> = saturdays
            .iter()
            .flat_map(|&s| [s, s.add_days(4)])
            .filter(|d| *d <= close)
            .collect();
        // Still short (very large leagues): fall back to every day.
        if both.len() < rounds {
            both = (0..=close.days_since(open) as u32)
                .map(|k| open.add_days(k))
                .collect();
        }
        both
    };
    assert!(
        pool.len() >= rounds,
        "cannot fit {rounds} rounds into a season"
    );
    (0..rounds).map(|i| pool[i * pool.len() / rounds]).collect()
}

/// A double round robin for `n` teams (circle method). Returns rounds;
/// each round is a list of `(home, away)` team indices. Every pair meets
/// twice, once at each ground, and home and away games alternate as
/// evenly as the format allows. With an odd `n`, one team rests each
/// round.
pub fn double_round_robin(n: usize) -> Vec<Vec<(usize, usize)>> {
    if n < 2 {
        return Vec::new();
    }
    let size = n + n % 2; // add a bye for odd counts
    let bye = if n % 2 == 1 { Some(n) } else { None };
    let mut ring: Vec<usize> = (0..size).collect();
    let mut first_half = Vec::with_capacity(size - 1);
    for round in 0..size - 1 {
        let mut games = Vec::with_capacity(size / 2);
        for i in 0..size / 2 {
            let a = ring[i];
            let b = ring[size - 1 - i];
            if Some(a) == bye || Some(b) == bye {
                continue;
            }
            // Alternate the fixed team's venue; flip pairs by position.
            let (h, w) = if i == 0 {
                if round % 2 == 0 { (a, b) } else { (b, a) }
            } else if i % 2 == 0 {
                (a, b)
            } else {
                (b, a)
            };
            games.push((h, w));
        }
        first_half.push(games);
        // Rotate everyone except the first team.
        let last = ring.pop().expect("non-empty");
        ring.insert(1, last);
    }
    let second_half: Vec<Vec<(usize, usize)>> = first_half
        .iter()
        .map(|r| r.iter().map(|&(h, a)| (a, h)).collect())
        .collect();
    first_half.into_iter().chain(second_half).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn season_dates_fit_the_window() {
        for rounds in [2, 18, 38, 46, 60] {
            let d = season_dates(2026, rounds);
            assert_eq!(d.len(), rounds);
            assert!(
                d.windows(2).all(|w| w[0] < w[1]),
                "{rounds} rounds not increasing"
            );
            assert!(d[0] >= season_opening(2026) && *d.last().unwrap() <= season_closing(2026));
        }
        let d = season_dates(2026, 38);
        assert!(d.iter().all(|x| x.weekday() == Weekday::Saturday));
        assert!(
            d.last().unwrap().ymd().1 == 5,
            "38 rounds should run into May: {}",
            d.last().unwrap()
        );
    }

    #[test]
    fn every_pair_meets_home_and_away() {
        for n in [2, 3, 4, 7, 20] {
            let rounds = double_round_robin(n);
            let mut count = vec![vec![0; n]; n];
            for r in &rounds {
                let mut playing = vec![false; n];
                for &(h, a) in r {
                    assert_ne!(h, a);
                    assert!(!playing[h] && !playing[a], "team plays twice in a round");
                    playing[h] = true;
                    playing[a] = true;
                    count[h][a] += 1;
                }
            }
            for (h, row) in count.iter().enumerate() {
                for (a, &c) in row.iter().enumerate() {
                    assert_eq!(c, usize::from(h != a), "n={n} {h}v{a}");
                }
            }
        }
    }

    #[test]
    fn home_games_are_balanced() {
        let rounds = double_round_robin(20);
        assert_eq!(rounds.len(), 38);
        let mut home = [0; 20];
        for r in &rounds {
            for &(h, _) in r {
                home[h] += 1;
            }
        }
        assert!(home.iter().all(|&h| h == 19));
    }
}
