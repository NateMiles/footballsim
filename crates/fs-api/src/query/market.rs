//! The transfer market, scouting and player search, from the viewer's
//! side.

use super::refs::*;
use crate::dto;
use fs_world::people::{age_years, wage_demand};
use fs_world::scouting;
use fs_world::staff::{self, StaffRole};
use fs_world::transfer::{self, Offer};
use fs_world::{ClubId, PlayerId, Status, World};

pub fn offer(world: &World, o: &Offer) -> dto::Offer {
    dto::Offer {
        id: o.id,
        date: date(o.date),
        player: player_ref(world, o.player),
        buyer: club_ref(world, o.buyer),
        seller: club_ref(world, o.seller),
        kind: o.kind.into(),
        wage: o.wage,
        years: o.years,
        sell_on: o.sell_on,
        status: o.status.into(),
        respond_by: date(o.respond_by),
    }
}

pub fn loan(world: &World, player: PlayerId, l: &transfer::Loan) -> dto::Loan {
    dto::Loan {
        player: player_ref(world, player),
        parent: club_ref(world, l.parent),
        borrower: club_ref(world, l.borrower),
        until: date(l.until),
        wage_share: l.wage_share,
    }
}

/// When the window next opens or closes.
fn window_changes(world: &World) -> String {
    let today = world.date();
    let mut d = today;
    let open = transfer::window_open(today);
    for _ in 0..366 {
        d = d.add_days(1);
        if transfer::window_open(d) != open {
            return date(d);
        }
    }
    date(d)
}

/// Recent moves shown on the transfers screen.
const RECENT: usize = 30;

pub fn transfers(world: &World, club: ClubId) -> dto::TransfersView {
    let m = world.market();
    let mut incoming: Vec<&Offer> = m.offers().iter().filter(|o| o.seller == club).collect();
    let mut outgoing: Vec<&Offer> = m.offers().iter().filter(|o| o.buyer == club).collect();
    incoming.reverse();
    outgoing.reverse();
    let today = world.date();
    let mut recent: Vec<&transfer::TransferRecord> = m
        .history()
        .iter()
        .filter(|r| today.days_since(r.date) <= 60)
        .collect();
    recent.sort_by_key(|r| (std::cmp::Reverse(r.fee), r.date));
    recent.truncate(RECENT);
    let ratings = staff::ratings(world, club);
    dto::TransfersView {
        window_open: transfer::window_open(today),
        window_changes: window_changes(world),
        transfer_budget: world.clubs()[club].finances().transfer_budget,
        incoming: incoming.iter().map(|o| offer(world, o)).collect(),
        outgoing: outgoing.iter().map(|o| offer(world, o)).collect(),
        listed: m
            .listed()
            .iter()
            .filter(|(p, _)| world.players().club(**p) == Some(club))
            .map(|(&p, &l)| dto::ListedPlayer {
                player: player_ref(world, p),
                listing: l.into(),
            })
            .collect(),
        loans_out: m
            .loans()
            .iter()
            .filter(|(_, l)| l.parent == club)
            .map(|(&p, l)| loan(world, p, l))
            .collect(),
        loans_in: m
            .loans()
            .iter()
            .filter(|(_, l)| l.borrower == club)
            .map(|(&p, l)| loan(world, p, l))
            .collect(),
        scouting: world
            .scouting()
            .assignments(club)
            .iter()
            .map(|&p| dto::Assignment {
                player: player_ref(world, p),
                club: world.players().club(p).map(|c| club_ref(world, c)),
                knowledge: scouting::knowledge_with(world, club, p, &ratings),
            })
            .collect(),
        scouts: world.clubs()[club]
            .staff()
            .iter()
            .filter(|&&s| world.staff()[s].role == StaffRole::Scout)
            .count() as u8,
        recent: recent
            .iter()
            .map(|r| dto::TransferRecord {
                date: date(r.date),
                player: player_ref(world, r.player),
                from: club_ref(world, r.from),
                to: club_ref(world, r.to),
                fee: r.fee,
                loan: r.loan,
            })
            .collect(),
    }
}

/// The value a club puts on a player, from its estimate of their ability.
pub fn estimated_value(world: &World, player: PlayerId, ability: f64, potential: f64) -> i64 {
    let ps = world.players();
    let age = age_years(ps.bio(player).birth_date, world.date());
    let years = ps.contract(player).map_or(0.0, |c| {
        c.expires.days_since(world.date()).max(0) as f64 / 365.25
    });
    transfer::valuation(ability, potential, age, years)
}

/// Most results a search returns.
pub const SEARCH_LIMIT: usize = 200;

/// Whether `hay` contains `needle` (already lower case), ignoring ASCII
/// case, without allocating.
fn contains_ci(hay: &str, needle: &str) -> bool {
    let (h, n) = (hay.as_bytes(), needle.as_bytes());
    n.is_empty()
        || h.windows(n.len())
            .any(|w| w.iter().zip(n).all(|(a, b)| a.to_ascii_lowercase() == *b))
}

/// A player who passed the filters, before the row is built.
struct Candidate {
    player: PlayerId,
    knowledge: u8,
    ability: scouting::Range,
    potential: scouting::Range,
    value: Option<i64>,
}

pub fn search(world: &World, club: Option<ClubId>, q: &dto::PlayerSearch) -> dto::SearchView {
    let today = world.date();
    let ratings = club.map(|c| staff::ratings(world, c));
    let ps = world.players();
    let name = q
        .name
        .as_ref()
        .map(|n| n.trim().to_lowercase())
        .filter(|n| !n.is_empty());
    let value_of = |p: PlayerId, ability: &scouting::Range, potential: &scouting::Range| {
        if ps.club(p).is_some() {
            estimated_value(
                world,
                p,
                f64::from(ability.value),
                f64::from(potential.value),
            )
        } else {
            0
        }
    };
    let mut found: Vec<Candidate> = Vec::new();
    for p in ps.ids() {
        // Cheap filters first.
        if ps.status(p) != Status::Active {
            continue;
        }
        let at = ps.club(p);
        if club.is_some() && at == club {
            continue;
        }
        if q.free_agents_only && at.is_some() {
            continue;
        }
        if q.listed_only && world.market().listing(p).is_none() {
            continue;
        }
        if q.min_age.is_some() || q.max_age.is_some() {
            let age = age_years(ps.bio(p).birth_date, today);
            if q.min_age.is_some_and(|a| age < f64::from(a))
                || q.max_age.is_some_and(|a| age >= f64::from(a) + 1.0)
            {
                continue;
            }
        }
        if !q.positions.is_empty() {
            let best = ps.positions(p).best().code();
            if !q.positions.iter().any(|c| c == best) {
                continue;
            }
        }
        if let Some(n) = &name {
            let bio = ps.bio(p);
            let (f, l) = (
                world.names().get(bio.first_name),
                world.names().get(bio.last_name),
            );
            let matched = if n.contains(' ') {
                format!("{f} {l}").to_lowercase().contains(n.as_str())
            } else {
                contains_ci(f, n) || contains_ci(l, n)
            };
            if !matched {
                continue;
            }
        }
        // Then what the viewer believes.
        let (knowledge, ability, potential) = match (club, &ratings) {
            (Some(c), Some(r)) => scouting::quick_estimate(world, c, p, r),
            _ => {
                let exact = |v: u16| scouting::Range {
                    value: v,
                    low: v,
                    high: v,
                };
                (
                    100,
                    exact(ps.ability(p)),
                    exact(ps.potential(p).max(ps.ability(p))),
                )
            }
        };
        if q.min_ability
            .is_some_and(|m| percent_of_1000(ability.value) < m)
        {
            continue;
        }
        let value = if q.max_value.is_some() {
            let v = value_of(p, &ability, &potential);
            if q.max_value.is_some_and(|m| v > m) {
                continue;
            }
            Some(v)
        } else {
            None
        };
        found.push(Candidate {
            player: p,
            knowledge,
            ability,
            potential,
            value,
        });
    }
    let total = found.len() as u32;
    let order = |a: &Candidate, b: &Candidate| {
        percent_of_1000(b.ability.value)
            .cmp(&percent_of_1000(a.ability.value))
            .then(percent_of_1000(b.potential.value).cmp(&percent_of_1000(a.potential.value)))
            .then(a.player.cmp(&b.player))
    };
    if found.len() > SEARCH_LIMIT {
        found.select_nth_unstable_by(SEARCH_LIMIT - 1, order);
        found.truncate(SEARCH_LIMIT);
    }
    found.sort_by(order);
    let rows = found
        .into_iter()
        .map(|c| {
            let p = c.player;
            let age = age_years(ps.bio(p).birth_date, today);
            let at = ps.club(p);
            dto::SearchRow {
                player: player_ref(world, p),
                club: at.map(|x| club_ref(world, x)),
                age: age as u8,
                nation: world.nations()[ps.bio(p).nation].code.clone(),
                ability: percent_of_1000(c.ability.value),
                ability_low: percent_of_1000(c.ability.low),
                ability_high: percent_of_1000(c.ability.high),
                potential: percent_of_1000(c.potential.value),
                knowledge: c.knowledge,
                value: c
                    .value
                    .unwrap_or_else(|| value_of(p, &c.ability, &c.potential)),
                wage_demand: wage_demand(ps.ability(p), ps.potential(p), age as u32),
                listing: world.market().listing(p).map(Into::into),
            }
        })
        .collect();
    dto::SearchView { rows, total }
}
