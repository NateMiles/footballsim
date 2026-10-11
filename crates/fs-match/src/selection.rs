//! Automatic team selection: the best available eleven for a formation,
//! plus a bench.

use crate::input::{PlayerSnapshot, Tactic, TeamSheet};
use crate::skills::familiarity_factor;
use fs_core::attributes::Position;

/// Maximum substitutes named.
pub const BENCH_SIZE: usize = 9;

/// How suitable a player is for a position right now: ability, familiarity
/// and fitness.
pub fn suitability(p: &PlayerSnapshot, position: Position) -> f64 {
    let ability = f64::from(position.role_rating(&p.attributes));
    let fit = f64::from(p.fitness) / 10_000.0;
    // Tired players are dropped sharply below about 85% fitness.
    let fitness_factor = if fit >= 0.85 {
        1.0
    } else {
        1.0 - (0.85 - fit) * 2.5
    };
    ability * familiarity_factor(p.positions.get(position)) * fitness_factor.max(0.1)
}

/// Picks a starting eleven and bench from `squad` for `tactic`.
///
/// Slots are filled scarcest-first (goalkeeper, then defenders, then
/// attackers), each taking the most suitable unpicked player. Squads with
/// fewer than eleven players are padded by reusing the weakest picks'
/// slots with no one; callers should not pass such squads.
///
/// # Panics
/// Panics if the squad has fewer than eleven players.
pub fn pick_team(squad: &[PlayerSnapshot], tactic: Tactic) -> TeamSheet {
    assert!(squad.len() >= 11, "need at least eleven players");
    let slots = tactic.formation.slots();
    let mut order: Vec<usize> = (0..11).collect();
    order.sort_by_key(|&i| scarcity(slots[i].position));

    // Every player's suitability for each position the formation uses,
    // computed once.
    let mut positions: Vec<Position> = Vec::with_capacity(11);
    let mut scores: Vec<Vec<f64>> = Vec::with_capacity(11);
    for s in &slots {
        if !positions.contains(&s.position) {
            positions.push(s.position);
            scores.push(squad.iter().map(|p| suitability(p, s.position)).collect());
        }
    }
    let mut taken = vec![false; squad.len()];
    let mut chosen: [Option<usize>; 11] = [None; 11];
    for &slot in &order {
        let pos = slots[slot].position;
        let score = &scores[positions.iter().position(|&p| p == pos).expect("listed")];
        let best = (0..squad.len())
            .filter(|&i| !taken[i])
            .max_by(|&a, &b| score[a].total_cmp(&score[b]).then(b.cmp(&a)))
            .expect("enough players");
        taken[best] = true;
        chosen[slot] = Some(best);
    }
    let starters = chosen
        .iter()
        .map(|c| squad[c.expect("filled")].clone())
        .collect();

    // Bench: a backup keeper if available, then the best remaining
    // outfielders by their best-position rating.
    let mut rest: Vec<usize> = (0..squad.len()).filter(|&i| !taken[i]).collect();
    let rating: Vec<f64> = squad.iter().map(best_rating).collect();
    rest.sort_by(|&a, &b| rating[b].total_cmp(&rating[a]).then(a.cmp(&b)));
    let mut bench = Vec::with_capacity(BENCH_SIZE);
    if let Some(k) = rest
        .iter()
        .position(|&i| squad[i].positions.best() == Position::Goalkeeper)
    {
        bench.push(squad[rest.remove(k)].clone());
    }
    for i in rest {
        if bench.len() == BENCH_SIZE {
            break;
        }
        if squad[i].positions.best() != Position::Goalkeeper {
            bench.push(squad[i].clone());
        }
    }
    TeamSheet {
        tactic,
        starters,
        bench,
        auto_subs: true,
        instructions: Default::default(),
    }
}

fn best_rating(p: &PlayerSnapshot) -> f64 {
    suitability(p, p.positions.best())
}

fn scarcity(p: Position) -> u8 {
    use Position::*;
    match p {
        Goalkeeper => 0,
        DefenderCentre => 1,
        DefenderLeft | DefenderRight => 2,
        WingBackLeft | WingBackRight => 3,
        DefensiveMidfielder => 4,
        Striker => 5,
        MidfielderCentre => 6,
        AttackingMidCentre => 7,
        MidfielderLeft | MidfielderRight | AttackingMidLeft | AttackingMidRight => 8,
    }
}
