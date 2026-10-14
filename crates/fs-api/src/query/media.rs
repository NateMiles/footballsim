//! The news written up, and the press room.
//!
//! Stories in the world are bare facts; this is where they get headlines,
//! a byline and a picture. Wording varies by story so a season of results
//! does not read like a list, but the same story always reads the same.

use super::refs::{club_ref, competition_ref, date, fixture, look, ordinal, player_ref};
use crate::dto::{self, Art, ArticleKind, Expression, Figure};
use crate::inbox::money;
use fs_world::news::{Angle as ResultAngle, Story, StoryKind, clubs_of};
use fs_world::press::{Conference, Effect, Occasion, Outlet, Quote, Tone, Topic, options};
use fs_world::rivals::Angle;
use fs_world::{ClubId, ManagerId, PlayerId, World};

/// One of `items`, the same every time for the same story and use.
fn pick<'a>(id: u32, salt: u32, items: &[&'a str]) -> &'a str {
    let h = id
        .wrapping_mul(2_654_435_761)
        .wrapping_add(salt.wrapping_mul(40_503));
    items[(h >> 7) as usize % items.len()]
}

/// `{a}`, `{b}`... replaced in order.
fn fill(template: &str, args: &[&str]) -> String {
    let mut out = template.to_owned();
    for (k, a) in args.iter().enumerate() {
        out = out.replace(&format!("{{{k}}}"), a);
    }
    out
}

fn club_name(world: &World, c: ClubId) -> String {
    world.clubs()[c].short_name.clone()
}

fn full_club_name(world: &World, c: ClubId) -> String {
    world.clubs()[c].name.clone()
}

fn player_name(world: &World, p: PlayerId) -> String {
    player_ref(world, p).name
}

fn manager_name(world: &World, m: ManagerId) -> String {
    let mg = &world.managers()[m];
    format!(
        "{} {}",
        world.names().get(mg.first_name),
        world.names().get(mg.last_name)
    )
}

fn manager_figure(world: &World, m: ManagerId, club: Option<ClubId>, e: Expression) -> Figure {
    Figure {
        name: manager_name(world, m),
        look: look(fs_world::looks::manager(world, m)),
        expression: e,
        club: club.map(|c| club_ref(world, c)),
    }
}

fn player_figure(world: &World, p: PlayerId, club: Option<ClubId>, e: Expression) -> Figure {
    Figure {
        name: player_name(world, p),
        look: look(fs_world::looks::player(world, p)),
        expression: e,
        club: club.map(|c| club_ref(world, c)),
    }
}

fn outlet_name(world: &World, nation: fs_world::NationId, outlet: Outlet) -> String {
    let n = &world.nations()[nation].name;
    match outlet {
        Outlet::Broadsheet => format!("The {n} Times"),
        Outlet::Tabloid => "The Daily Shout".to_owned(),
        Outlet::Broadcaster => format!("{n} Sport"),
        Outlet::FanSite => "Terrace Talk".to_owned(),
    }
}

fn outlet_kind(o: Outlet) -> &'static str {
    match o {
        Outlet::Broadsheet => "broadsheet",
        Outlet::Tabloid => "tabloid",
        Outlet::Broadcaster => "broadcaster",
        Outlet::FanSite => "fanSite",
    }
}

fn journalist_name(world: &World, j: &fs_world::press::Journalist) -> String {
    format!(
        "{} {}",
        world.names().get(j.first_name),
        world.names().get(j.last_name)
    )
}

fn journalist_look(world: &World, index: u32) -> dto::Look {
    look(fs_world::looks::look(
        world.seed(),
        fs_world::looks::Who::Journalist(index),
        world.press().journalist(index).map(|j| j.nation),
        28.0 + f64::from(index % 7) * 5.0,
    ))
}

fn byline(world: &World, story: &Story) -> dto::Byline {
    let nation = clubs_of(&story.kind)
        .into_iter()
        .flatten()
        .next()
        .map(|c| world.clubs()[c].nation);
    let found: Vec<(u32, &fs_world::press::Journalist)> = world
        .press()
        .journalists()
        .iter()
        .enumerate()
        .filter(|(_, j)| Some(j.nation) == nation)
        .map(|(i, j)| (i as u32, j))
        .collect();
    // Rows to the tabloid, results to the broadcaster, the rest shared.
    let wanted = match story.kind {
        StoryKind::Quote { .. } | StoryKind::TransferRequest { .. } | StoryKind::Rumour { .. } => {
            Some(Outlet::Tabloid)
        }
        StoryKind::Result { .. } | StoryKind::HatTrick { .. } => Some(Outlet::Broadcaster),
        StoryKind::Wonderkid { .. } => Some(Outlet::FanSite),
        _ => None,
    };
    let chosen = wanted
        .and_then(|o| found.iter().find(|(_, j)| j.outlet == o))
        .or_else(|| found.get(story.id as usize % found.len().max(1)));
    match chosen {
        Some(&(i, j)) => dto::Byline {
            name: journalist_name(world, j),
            outlet: outlet_name(world, j.nation, j.outlet),
            look: Some(journalist_look(world, i)),
        },
        None => dto::Byline {
            name: "Staff reporter".to_owned(),
            outlet: "Football Wire".to_owned(),
            look: None,
        },
    }
}

/// The table position of a club in its league, as `"3rd"`.
fn position(world: &World, club: ClubId) -> Option<(String, String)> {
    let league = world.clubs()[club].league()?;
    let season = world.leagues()[league].season()?;
    let table = season.table();
    let pos = table.iter().position(|r| r.club == club)?;
    Some((
        ordinal((pos + 1) as u8),
        world.leagues()[league].name.clone(),
    ))
}

/// Writes one story.
pub fn article(world: &World, story: &Story, viewer: Option<ClubId>) -> dto::Article {
    let id = story.id;
    let mine = viewer.is_some_and(|v| clubs_of(&story.kind).contains(&Some(v)));
    let mut player = None;
    let (kind, headline, standfirst, body, art) = match story.kind {
        StoryKind::Result {
            fixture: r,
            home,
            away,
            goals,
            angle,
        } => {
            let comp = competition_ref(world, r).name;
            let f = fixture(world, r);
            let (h, a) = (club_name(world, home), club_name(world, away));
            let score = format!("{}-{}", goals[0], goals[1]);
            let (w, l, ws, ls) = if goals[0] >= goals[1] {
                (h.clone(), a.clone(), goals[0], goals[1])
            } else {
                (a.clone(), h.clone(), goals[1], goals[0])
            };
            let wl = format!("{ws}-{ls}");
            let draw = goals[0] == goals[1];
            let headline = match angle {
                ResultAngle::Upset => fill(
                    pick(
                        id,
                        1,
                        &[
                            "{0} stun {1}",
                            "Giant-killing: {0} topple {1}",
                            "{1} humbled by {0}",
                            "Shock as {0} beat {1}",
                        ],
                    ),
                    &[&w, &l],
                ),
                ResultAngle::Thrashing => fill(
                    pick(
                        id,
                        1,
                        &[
                            "{0} run riot against {1}",
                            "{1} torn apart by rampant {0}",
                            "{0} hit {2} past sorry {1}",
                        ],
                    ),
                    &[&w, &l, &ws.to_string()],
                ),
                ResultAngle::Summit if draw => fill(
                    pick(
                        id,
                        1,
                        &[
                            "Honours even at the top",
                            "{0} and {1} cancel each other out",
                        ],
                    ),
                    &[&h, &a],
                ),
                ResultAngle::Summit => fill(
                    pick(
                        id,
                        1,
                        &[
                            "{0} win the battle at the top",
                            "{0} edge {1} in summit meeting",
                            "Advantage {0} after top-of-the-table clash",
                        ],
                    ),
                    &[&w, &l],
                ),
                ResultAngle::GoalFest => fill(
                    pick(
                        id,
                        1,
                        &[
                            "{0} {2} {1}: a game for the ages",
                            "Goals galore as {0} and {1} share {3}",
                        ],
                    ),
                    &[&h, &a, &score, &(goals[0] + goals[1]).to_string()],
                ),
                ResultAngle::Report if draw => fill(
                    pick(
                        id,
                        1,
                        &[
                            "{0} and {1} share the points",
                            "Stalemate between {0} and {1}",
                            "{0} held by {1}",
                        ],
                    ),
                    &[&h, &a],
                ),
                ResultAngle::Report => fill(
                    pick(
                        id,
                        1,
                        &["{0} beat {1}", "Win for {0} against {1}", "{1} fall to {0}"],
                    ),
                    &[&w, &l],
                ),
            };
            let standfirst = if draw {
                format!(
                    "{} and {} drew {score} in the {comp}.",
                    full_club_name(world, home),
                    full_club_name(world, away)
                )
            } else {
                let (wf, lf) = if goals[0] > goals[1] {
                    (full_club_name(world, home), full_club_name(world, away))
                } else {
                    (full_club_name(world, away), full_club_name(world, home))
                };
                format!("{wf} beat {lf} {wl} in the {comp}.")
            };
            let mut body = Vec::new();
            if let Some(res) = &f.result
                && !res.scorers.is_empty()
            {
                let names: Vec<String> = res
                    .scorers
                    .iter()
                    .map(|s| s.player.short_name.clone())
                    .collect();
                body.push(format!("Goals: {}.", names.join(", ")));
            }
            if angle == ResultAngle::Upset {
                body.push(fill(
                    pick(id, 2, &[
                        "Few gave {0} a chance against a club of {1}'s standing, but they were the better side on the day.",
                        "It is the kind of result that lives long in the memory for {0}, and one {1} will want to forget quickly.",
                    ]),
                    &[&w, &l],
                ));
            }
            for c in [home, away] {
                if let (fs_world::FixtureRef::League { .. }, Some((pos, league))) =
                    (r, position(world, c))
                {
                    body.push(format!(
                        "{} are {pos} in the {league}.",
                        club_name(world, c)
                    ));
                }
            }
            let art = Art::Score {
                home: club_ref(world, home),
                away: club_ref(world, away),
                goals,
                competition: comp,
            };
            (ArticleKind::Result, headline, standfirst, body, art)
        }
        StoryKind::HatTrick {
            player: p,
            club,
            against,
            goals,
        } => {
            player = Some(p.raw());
            let name = player_name(world, p);
            let n = if goals == 3 {
                "a hat-trick".to_owned()
            } else {
                format!("{goals} goals")
            };
            let headline = fill(
                pick(
                    id,
                    1,
                    &[
                        "{0} hat-trick sinks {1}",
                        "Three and easy for {0}",
                        "{0} fires {2} to victory",
                        "Match ball for {0}",
                    ],
                ),
                &[&name, &club_name(world, against), &club_name(world, club)],
            );
            let standfirst = format!(
                "{name} scored {n} for {} against {}.",
                full_club_name(world, club),
                full_club_name(world, against)
            );
            let s = world.players().season_stats(p);
            let body = vec![format!(
                "That takes {name} to {} goal{} in {} appearance{} this season.",
                s.goals,
                if s.goals == 1 { "" } else { "s" },
                s.appearances,
                if s.appearances == 1 { "" } else { "s" }
            )];
            let art = Art::Portrait {
                figure: player_figure(world, p, Some(club), Expression::Delighted),
                caption: format!("{name} celebrates"),
            };
            (ArticleKind::Result, headline, standfirst, body, art)
        }
        StoryKind::Transfer {
            player: p,
            from,
            to,
            fee,
            loan,
            record,
        } => {
            player = Some(p.raw());
            let name = player_name(world, p);
            let to_n = club_name(world, to);
            let from_n = from.map(|c| club_name(world, c)).unwrap_or_default();
            let fee_s = money(fee);
            let headline = if record {
                fill(
                    pick(
                        id,
                        1,
                        &[
                            "World record! {0} pay {1} for {2}",
                            "{2} becomes the world's most expensive player",
                        ],
                    ),
                    &[&to_n, &fee_s, &name],
                )
            } else if loan {
                fill(
                    pick(id, 1, &["{0} joins {1} on loan", "{1} borrow {0} from {2}"]),
                    &[&name, &to_n, &from_n],
                )
            } else {
                fill(
                    pick(
                        id,
                        1,
                        &[
                            "{1} sign {0} from {2}",
                            "{0} completes {3} move to {1}",
                            "{1} land {0} in {3} deal",
                            "Done deal: {0} joins {1}",
                        ],
                    ),
                    &[&name, &to_n, &from_n, &fee_s],
                )
            };
            let age = super::refs::age(world, p);
            let pos = player_ref(world, p).position;
            let standfirst = if loan {
                format!(
                    "The {age}-year-old {pos} will spend the rest of the season at {}.",
                    full_club_name(world, to)
                )
            } else {
                format!(
                    "The {age}-year-old {pos} has signed for {} for a fee of {fee_s}.",
                    full_club_name(world, to)
                )
            };
            let s = world.players().season_stats(p);
            let mut body = Vec::new();
            if let Some(f) = from {
                body.push(format!(
                    "{name} leaves {} after {} appearance{} this season.",
                    full_club_name(world, f),
                    s.appearances,
                    if s.appearances == 1 { "" } else { "s" }
                ));
            }
            if let Some(m) = world.clubs()[to].manager {
                body.push(format!(
                    "{} {}",
                    manager_name(world, m),
                    pick(
                        id,
                        2,
                        &[
                            "called the signing \"a statement of intent\".",
                            "said the club had \"wanted this one for a long time\".",
                            "is expected to hand the new arrival a debut straight away.",
                        ]
                    )
                ));
            }
            let art = Art::Move {
                figure: player_figure(world, p, Some(to), Expression::Happy),
                from: from.map(|c| club_ref(world, c)),
                to: club_ref(world, to),
                fee: if loan { "Loan".to_owned() } else { fee_s },
            };
            (ArticleKind::Transfer, headline, standfirst, body, art)
        }
        StoryKind::Rumour {
            player: p,
            club,
            owner,
        } => {
            player = Some(p.raw());
            let name = player_name(world, p);
            let headline = fill(
                pick(
                    id,
                    1,
                    &[
                        "{1} make move for {2}'s {0}",
                        "{0} wanted by {1}",
                        "{1} test {2} resolve with bid for {0}",
                    ],
                ),
                &[&name, &club_name(world, club), &club_name(world, owner)],
            );
            let standfirst = format!(
                "{} have made an offer for {name}, sources close to the club say.",
                full_club_name(world, club)
            );
            let body = vec![format!(
                "{} are yet to respond. {name} is under contract{}.",
                full_club_name(world, owner),
                world
                    .players()
                    .contract(p)
                    .map(|c| format!(" until {}", c.expires.ymd().0))
                    .unwrap_or_default()
            )];
            let art = Art::Move {
                figure: player_figure(world, p, Some(owner), Expression::Sceptical),
                from: Some(club_ref(world, owner)),
                to: club_ref(world, club),
                fee: "Rumour".to_owned(),
            };
            (ArticleKind::Rumour, headline, standfirst, body, art)
        }
        StoryKind::ManagerSacked { manager, club } => {
            let m = manager_name(world, manager);
            let c = club_name(world, club);
            let headline = fill(
                pick(
                    id,
                    1,
                    &[
                        "{1} sack {0}",
                        "{0} pays the price at {1}",
                        "{0} out at {1}",
                    ],
                ),
                &[&m, &c],
            );
            let standfirst = format!(
                "{} have parted company with {m} after a run of poor results.",
                full_club_name(world, club)
            );
            let mut body = Vec::new();
            if let Some((pos, league)) = position(world, club) {
                body.push(format!("{c} are {pos} in the {league}."));
            }
            body.push(
                pick(
                    id,
                    2,
                    &[
                        "The board will begin the search for a successor immediately.",
                        "A caretaker will take charge while the club looks for a replacement.",
                    ],
                )
                .to_owned(),
            );
            let art = Art::Portrait {
                figure: manager_figure(world, manager, Some(club), Expression::Sad),
                caption: format!("{m} leaves {c}"),
            };
            (ArticleKind::Manager, headline, standfirst, body, art)
        }
        StoryKind::ManagerAppointed { manager, club } => {
            let m = manager_name(world, manager);
            let c = club_name(world, club);
            let headline = fill(
                pick(
                    id,
                    1,
                    &[
                        "{0} takes charge at {1}",
                        "{1} appoint {0}",
                        "New era at {1} as {0} arrives",
                    ],
                ),
                &[&m, &c],
            );
            let standfirst = format!("{m} is the new manager of {}.", full_club_name(world, club));
            let body = vec![
                pick(id, 2, &[
                    "\"It is a huge honour,\" the new manager said. \"I can't wait to get started.\"",
                    "The appointment ends the club's search for a new manager.",
                ])
                .to_owned(),
            ];
            let art = Art::Portrait {
                figure: manager_figure(world, manager, Some(club), Expression::Happy),
                caption: format!("{m}, {c}'s new manager"),
            };
            (ArticleKind::Manager, headline, standfirst, body, art)
        }
        StoryKind::UnderPressure { manager, club } => {
            let m = manager_name(world, manager);
            let c = club_name(world, club);
            let headline = fill(
                pick(
                    id,
                    1,
                    &[
                        "Pressure mounts on {0}",
                        "{1} board losing patience with {0}",
                        "{0} under fire at {1}",
                    ],
                ),
                &[&m, &c],
            );
            let standfirst = format!("Results have left {m}'s position at {c} in doubt.");
            let mut body = Vec::new();
            if let Some((pos, league)) = position(world, club) {
                body.push(format!(
                    "{c} sit {pos} in the {league}, well short of what the board expected."
                ));
            }
            let art = Art::Portrait {
                figure: manager_figure(world, manager, Some(club), Expression::Annoyed),
                caption: m.clone(),
            };
            (ArticleKind::Manager, headline, standfirst, body, art)
        }
        StoryKind::CupWon { cup, club } => {
            let cup_name = world.cups()[cup].format.name.clone();
            let c = club_name(world, club);
            let headline = fill(
                pick(
                    id,
                    1,
                    &[
                        "{0} lift the {1}!",
                        "Glory for {0} in the {1}",
                        "{1} winners: {0}",
                    ],
                ),
                &[&c, &cup_name],
            );
            let standfirst = format!(
                "{} are the {cup_name} winners.",
                full_club_name(world, club)
            );
            let art = Art::Trophy {
                club: club_ref(world, club),
                competition: cup_name,
            };
            (ArticleKind::Trophy, headline, standfirst, Vec::new(), art)
        }
        StoryKind::Champions { league, club } => {
            let l = world.leagues()[league].name.clone();
            let c = club_name(world, club);
            let headline = fill(
                pick(
                    id,
                    1,
                    &[
                        "{0} crowned {1} champions",
                        "Champions! {0} win the {1}",
                        "The title is {0}'s",
                    ],
                ),
                &[&c, &l],
            );
            let standfirst = format!("{} have won the {l}.", full_club_name(world, club));
            let art = Art::Trophy {
                club: club_ref(world, club),
                competition: l,
            };
            (ArticleKind::Trophy, headline, standfirst, Vec::new(), art)
        }
        StoryKind::Promoted { club, to } | StoryKind::Relegated { club, to } => {
            let up = matches!(story.kind, StoryKind::Promoted { .. });
            let l = world.leagues()[to].name.clone();
            let c = club_name(world, club);
            let headline = if up {
                fill(
                    pick(
                        id,
                        1,
                        &["{0} promoted to the {1}", "Up they go: {0} reach the {1}"],
                    ),
                    &[&c, &l],
                )
            } else {
                fill(
                    pick(
                        id,
                        1,
                        &["{0} relegated to the {1}", "Heartbreak as {0} go down"],
                    ),
                    &[&c, &l],
                )
            };
            let standfirst = format!(
                "{} will play in the {l} next season.",
                full_club_name(world, club)
            );
            let art = if up {
                Art::Trophy {
                    club: club_ref(world, club),
                    competition: format!("Promoted to the {l}"),
                }
            } else {
                Art::Score {
                    home: club_ref(world, club),
                    away: club_ref(world, club),
                    goals: [0, 0],
                    competition: format!("Relegated to the {l}"),
                }
            };
            (ArticleKind::Trophy, headline, standfirst, Vec::new(), art)
        }
        StoryKind::LongInjury {
            player: p,
            club,
            days,
        } => {
            player = Some(p.raw());
            let name = player_name(world, p);
            let weeks = days.div_ceil(7);
            let headline = fill(
                pick(
                    id,
                    1,
                    &[
                        "Blow for {1}: {0} out for {2} weeks",
                        "{0} faces {2} weeks out",
                        "{1} sweat on {0} injury",
                    ],
                ),
                &[&name, &club_name(world, club), &weeks.to_string()],
            );
            let what = match world.players().injury(p).map(|i| i.severity) {
                Some(fs_world::player::Severity::Serious) => "a serious injury",
                _ => "an injury",
            };
            let standfirst = format!("{name} will miss around {weeks} weeks with {what}.");
            let art = Art::Portrait {
                figure: player_figure(world, p, Some(club), Expression::Sad),
                caption: format!("{name} is out"),
            };
            (ArticleKind::Injury, headline, standfirst, Vec::new(), art)
        }
        StoryKind::TransferRequest { player: p, club } => {
            player = Some(p.raw());
            let name = player_name(world, p);
            let c = club_name(world, club);
            let headline = fill(
                pick(
                    id,
                    1,
                    &[
                        "{0} asks to leave {1}",
                        "{0} hands in transfer request",
                        "Unrest at {1} as {0} wants out",
                    ],
                ),
                &[&name, &c],
            );
            let standfirst = format!(
                "{name} has told {} they want to move on.",
                full_club_name(world, club)
            );
            let art = Art::Portrait {
                figure: player_figure(world, p, Some(club), Expression::Annoyed),
                caption: name.clone(),
            };
            (ArticleKind::Unrest, headline, standfirst, Vec::new(), art)
        }
        StoryKind::Retired { player: p, club } => {
            player = Some(p.raw());
            let name = player_name(world, p);
            let headline = fill(
                pick(
                    id,
                    1,
                    &[
                        "{0} calls time on career",
                        "{0} retires",
                        "End of an era: {0} retires",
                    ],
                ),
                &[&name],
            );
            let career = world.players().career(p);
            let apps: u32 = career.iter().map(|s| u32::from(s.stats.appearances)).sum();
            let goals: u32 = career.iter().map(|s| u32::from(s.stats.goals)).sum();
            let standfirst = format!("{name} retires after {apps} appearances and {goals} goals.");
            let art = Art::Portrait {
                figure: player_figure(world, p, club, Expression::Happy),
                caption: name.clone(),
            };
            (
                ArticleKind::Retirement,
                headline,
                standfirst,
                Vec::new(),
                art,
            )
        }
        StoryKind::Wonderkid { player: p, club } => {
            player = Some(p.raw());
            let name = player_name(world, p);
            let age = super::refs::age(world, p);
            let headline = fill(
                pick(
                    id,
                    1,
                    &[
                        "{1} unearth a gem in {0}",
                        "Meet {0}, {1}'s {2}-year-old prodigy",
                        "The next big thing? {1}'s {0}",
                    ],
                ),
                &[&name, &club_name(world, club), &age.to_string()],
            );
            let standfirst = format!(
                "Scouts are already talking about {name}, the pick of this year's {} youth intake.",
                full_club_name(world, club)
            );
            let art = Art::Portrait {
                figure: player_figure(world, p, Some(club), Expression::Delighted),
                caption: format!("{name}, {age}"),
            };
            (ArticleKind::Youth, headline, standfirst, Vec::new(), art)
        }
        StoryKind::Quote {
            manager,
            club,
            quote,
        } => {
            let m = manager_name(world, manager);
            let (headline, standfirst, body) = quote_story(world, id, manager, club, quote);
            if let Some(p) = quote.topic.player() {
                player = Some(p.raw());
            }
            let expression = match quote.tone {
                Tone::Confident | Tone::Praise => Expression::Happy,
                Tone::Dismissive => Expression::Sceptical,
                Tone::Defiant => Expression::Determined,
                Tone::Criticise => Expression::Angry,
                Tone::Humble | Tone::Cautious => Expression::Neutral,
                Tone::Deflect => Expression::Annoyed,
            };
            let art = if quote.topic == Topic::Snub {
                Art::Portrait {
                    figure: manager_figure(world, manager, Some(club), Expression::Annoyed),
                    caption: format!("{m} stayed away"),
                }
            } else {
                Art::Microphone {
                    figure: manager_figure(world, manager, Some(club), expression),
                }
            };
            (ArticleKind::Quote, headline, standfirst, body, art)
        }
    };
    dto::Article {
        id,
        date: date(story.date),
        kind,
        headline,
        standfirst,
        body,
        byline: byline(world, story),
        art,
        weight: story.weight,
        mine,
        player,
    }
}

fn quote_story(
    world: &World,
    id: u32,
    manager: ManagerId,
    club: ClubId,
    quote: Quote,
) -> (String, String, Vec<String>) {
    let m = manager_name(world, manager);
    let c = club_name(world, club);
    let said = answer_text(world, club, quote.topic, quote.tone);
    let short = said
        .split(['.', '!', '?'])
        .next()
        .unwrap_or(&said)
        .trim()
        .to_owned();
    let p = |pl: PlayerId| player_name(world, pl);
    let headline = match (quote.topic, quote.tone) {
        (Topic::AboutRival { rival, angle }, tone) => {
            let r = club_name(world, rival);
            match (angle, tone) {
                (Angle::Reply, Tone::Dismissive) => fill(
                    pick(
                        id,
                        1,
                        &[
                            "{0} hits back at {1}",
                            "War of words: {0} fires back at {1}",
                        ],
                    ),
                    &[&m, &r],
                ),
                (Angle::AfterMeeting { .. }, Tone::Praise) => fill(
                    pick(
                        id,
                        1,
                        &[
                            "{0} hails {1} after defeat",
                            "\"Too good for us\" - {0} on {1}",
                        ],
                    ),
                    &[&m, &r],
                ),
                (Angle::AfterMeeting { .. }, Tone::Defiant) => fill(
                    pick(
                        id,
                        1,
                        &[
                            "{0}: {1} were lucky",
                            "Sour grapes? {0} says {1} were lucky",
                        ],
                    ),
                    &[&m, &r],
                ),
                (Angle::AfterMeeting { .. }, Tone::Dismissive) => fill(
                    pick(
                        id,
                        1,
                        &["{0} rubs it in after beating {1}", "{0} twists the knife"],
                    ),
                    &[&m, &r],
                ),
                (Angle::Race, Tone::Dismissive) => fill(
                    pick(
                        id,
                        1,
                        &[
                            "{0}: {1} will crack",
                            "Mind games: {0} predicts {1} collapse",
                        ],
                    ),
                    &[&m, &r],
                ),
                (Angle::Race, _) => fill(
                    pick(
                        id,
                        1,
                        &[
                            "{0} lays down a marker for {1}",
                            "Title talk: {0} sends message to {1}",
                        ],
                    ),
                    &[&m, &r],
                ),
                (Angle::Slump, _) => fill(
                    pick(
                        id,
                        1,
                        &[
                            "{0} mocks struggling {1}",
                            "{0} kicks {1} while they are down",
                        ],
                    ),
                    &[&m, &r],
                ),
                _ => fill(
                    pick(id, 1, &["\"{1}\" - {0}", "{0}: \"{1}\""]),
                    &[&m, &short],
                ),
            }
        }
        (Topic::RivalComment { rival, .. }, Tone::Dismissive)
        | (Topic::Grudge { opponent: rival }, Tone::Dismissive) => fill(
            pick(
                id,
                1,
                &["{0} fires back at {1}", "No love lost: {0} hits out at {1}"],
            ),
            &[&m, &rival_boss(world, rival)],
        ),
        (Topic::Snub, _) => fill(
            pick(
                id,
                1,
                &[
                    "{0} snubs the press",
                    "Empty chair as {0} skips press conference",
                ],
            ),
            &[&m],
        ),
        (Topic::Preview { opponent }, Tone::Dismissive)
        | (Topic::RivalManager { opponent, .. }, Tone::Dismissive) => fill(
            pick(
                id,
                1,
                &[
                    "{0} fires a warning to {1}",
                    "\"{2}\" - {0} goads {1}",
                    "{0} plays mind games with {1}",
                ],
            ),
            &[&m, &club_name(world, opponent), &short],
        ),
        (Topic::Preview { opponent }, Tone::Humble) => fill(
            pick(id, 1, &["{0} wary of {1}", "{0} full of respect for {1}"]),
            &[&m, &club_name(world, opponent)],
        ),
        (Topic::Result { .. }, Tone::Criticise) => fill(
            pick(
                id,
                1,
                &[
                    "{0} slams \"unacceptable\" {1}",
                    "{0} lays into {1} players",
                ],
            ),
            &[&m, &c],
        ),
        (Topic::PoorForm { player }, Tone::Criticise) => fill(
            pick(
                id,
                1,
                &["{0} warns {1}: not good enough", "{0} turns on {1}"],
            ),
            &[&m, &p(player)],
        ),
        (Topic::Expectations, Tone::Confident) => fill(
            pick(
                id,
                1,
                &[
                    "{0}: we're here to win things",
                    "{0} sets the bar high at {1}",
                ],
            ),
            &[&m, &c],
        ),
        (Topic::StarForm { player }, Tone::Praise)
        | (Topic::KeyPlayer { player }, Tone::Praise)
        | (Topic::NewSigning { player }, Tone::Praise)
        | (Topic::Youngster { player }, Tone::Praise) => fill(
            pick(
                id,
                1,
                &["{0} hails \"special\" {1}", "{0}: {1} is the real deal"],
            ),
            &[&m, &p(player)],
        ),
        (Topic::Rumour { player, .. }, Tone::Defiant)
        | (Topic::Unrest { player }, Tone::Defiant) => fill(
            pick(
                id,
                1,
                &[
                    "\"{1} is not for sale\" - {0}",
                    "{0} slams door on {1} exit",
                ],
            ),
            &[&m, &p(player)],
        ),
        (Topic::Rumour { player, .. }, Tone::Cautious) => fill(
            pick(
                id,
                1,
                &[
                    "{0} admits {1} could leave",
                    "{1} exit door left open by {0}",
                ],
            ),
            &[&m, &p(player)],
        ),
        (Topic::Unrest { .. }, Tone::Criticise) => fill(
            pick(
                id,
                1,
                &[
                    "\"No player is bigger than {1}\" - {0}",
                    "{0} takes hard line on unrest",
                ],
            ),
            &[&m, &c],
        ),
        (Topic::TitleRace, Tone::Confident) => fill(
            pick(
                id,
                1,
                &[
                    "{0}: we can win the league",
                    "{0} talks up {1}'s title chances",
                ],
            ),
            &[&m, &c],
        ),
        (Topic::Pressure, Tone::Defiant) | (Topic::Relegation, Tone::Defiant) => fill(
            pick(
                id,
                1,
                &[
                    "Defiant {0} vows to fight on",
                    "\"Write us off at your peril\" - {0}",
                ],
            ),
            &[&m],
        ),
        _ => fill(
            pick(id, 1, &["\"{1}\" - {0}", "{0}: \"{1}\""]),
            &[&m, &short],
        ),
    };
    if quote.topic == Topic::Snub {
        return (
            headline,
            format!("{m} did not appear at {c}'s press conference."),
            vec!["Journalists were left waiting and the club offered no explanation.".to_owned()],
        );
    }
    let when = match quote.topic {
        Topic::Result { opponent, goals } => format!(
            "Speaking after the {}-{} {} {}",
            goals[0],
            goals[1],
            if goals[0] > goals[1] {
                "win over"
            } else if goals[0] < goals[1] {
                "defeat to"
            } else {
                "draw with"
            },
            club_name(world, opponent)
        ),
        Topic::Preview { opponent }
        | Topic::RivalManager { opponent, .. }
        | Topic::Grudge { opponent } => {
            format!(
                "Speaking ahead of the game against {}",
                club_name(world, opponent)
            )
        }
        Topic::AboutRival { rival, .. } => format!("Asked about {}", club_name(world, rival)),
        Topic::RivalComment { rival, .. } => {
            format!("Responding to {}", rival_boss(world, rival))
        }
        _ => format!("Speaking at {c}'s press conference"),
    };
    let standfirst = format!("{when}, {m} did not hold back.");
    (
        headline,
        standfirst,
        vec![format!("{when}, {m} said: \"{said}\"")],
    )
}

/// The news for a viewer.
pub fn news(world: &World, viewer: Option<ClubId>, scope: dto::NewsScope) -> dto::NewsView {
    let nation = viewer.map(|c| world.clubs()[c].nation);
    let articles = world
        .news()
        .stories()
        .iter()
        .rev()
        .filter(|s| {
            let clubs = clubs_of(&s.kind);
            let ours = viewer.is_some_and(|v| clubs.contains(&Some(v)));
            match scope {
                dto::NewsScope::Club => ours,
                dto::NewsScope::Nation => {
                    ours || clubs
                        .iter()
                        .flatten()
                        .any(|&c| Some(world.clubs()[c].nation) == nation)
                }
                dto::NewsScope::World => ours || s.weight >= 60,
            }
        })
        .take(60)
        .map(|s| article(world, s, viewer))
        .collect();
    dto::NewsView { scope, articles }
}

/// The few biggest stories of the last week, for the front page.
pub fn headlines(world: &World, viewer: Option<ClubId>, n: usize) -> Vec<dto::Article> {
    let today = world.date();
    let nation = viewer.map(|c| world.clubs()[c].nation);
    let mut recent: Vec<&Story> = world
        .news()
        .stories()
        .iter()
        .filter(|s| today.days_since(s.date) <= 7)
        .collect();
    let score = |s: &Story| {
        let clubs = clubs_of(&s.kind);
        let ours = viewer.is_some_and(|v| clubs.contains(&Some(v)));
        let home = clubs
            .iter()
            .flatten()
            .any(|&c| Some(world.clubs()[c].nation) == nation);
        u32::from(s.weight) + if ours { 40 } else { 0 } + if home { 15 } else { 0 }
            - (today.days_since(s.date) as u32) * 3
    };
    recent.sort_by(|a, b| score(b).cmp(&score(a)).then(b.id.cmp(&a.id)));
    recent
        .into_iter()
        .take(n)
        .map(|s| article(world, s, viewer))
        .collect()
}

// The press room.

fn tone_dto(t: Tone) -> dto::Tone {
    match t {
        Tone::Confident => dto::Tone::Confident,
        Tone::Cautious => dto::Tone::Cautious,
        Tone::Praise => dto::Tone::Praise,
        Tone::Criticise => dto::Tone::Criticise,
        Tone::Deflect => dto::Tone::Deflect,
        Tone::Defiant => dto::Tone::Defiant,
        Tone::Humble => dto::Tone::Humble,
        Tone::Dismissive => dto::Tone::Dismissive,
    }
}

pub fn tone_from(t: dto::Tone) -> Tone {
    match t {
        dto::Tone::Confident => Tone::Confident,
        dto::Tone::Cautious => Tone::Cautious,
        dto::Tone::Praise => Tone::Praise,
        dto::Tone::Criticise => Tone::Criticise,
        dto::Tone::Deflect => Tone::Deflect,
        dto::Tone::Defiant => Tone::Defiant,
        dto::Tone::Humble => Tone::Humble,
        dto::Tone::Dismissive => Tone::Dismissive,
    }
}

fn tone_label(t: Tone) -> &'static str {
    match t {
        Tone::Confident => "Confident",
        Tone::Cautious => "Cautious",
        Tone::Praise => "Praise",
        Tone::Criticise => "Criticise",
        Tone::Deflect => "No comment",
        Tone::Defiant => "Defiant",
        Tone::Humble => "Respectful",
        Tone::Dismissive => "Dismissive",
    }
}

/// The question as the journalist puts it.
fn question_text(world: &World, club: ClubId, topic: Topic, salt: u32) -> String {
    let c = club_name(world, club);
    let p = |pl: PlayerId| player_name(world, pl);
    match topic {
        Topic::Preview { opponent } => fill(
            pick(
                salt,
                3,
                &[
                    "{0} next. How do you see the game?",
                    "What are you expecting from {0}?",
                    "Big game against {0}. Are your players ready?",
                ],
            ),
            &[&club_name(world, opponent)],
        ),
        Topic::RivalManager { opponent, tone } => {
            let said = match tone {
                Tone::Dismissive => "says they don't fear you at all",
                Tone::Humble => "says you're the favourites",
                _ => "says they're coming to win",
            };
            format!(
                "{}'s manager {said}. Your response?",
                club_name(world, opponent)
            )
        }
        Topic::Result { opponent, goals } => {
            let o = club_name(world, opponent);
            let s = format!("{}-{}", goals[0], goals[1]);
            if goals[0] > goals[1] {
                fill(
                    pick(
                        salt,
                        3,
                        &[
                            "A {0} win over {1}. Pleased?",
                            "{0} against {1}. How good was that?",
                        ],
                    ),
                    &[&s, &o],
                )
            } else if goals[0] == goals[1] {
                fill(
                    pick(
                        salt,
                        3,
                        &[
                            "A {0} draw with {1}. Fair result?",
                            "{0} with {1}. Two points dropped?",
                        ],
                    ),
                    &[&s, &o],
                )
            } else {
                fill(
                    pick(
                        salt,
                        3,
                        &[
                            "A {0} defeat to {1}. What went wrong?",
                            "Beaten {0} by {1}. Your thoughts?",
                        ],
                    ),
                    &[&s, &o],
                )
            }
        }
        Topic::StarForm { player } => {
            format!("{} is in brilliant form. How good can they be?", p(player))
        }
        Topic::PoorForm { player } => {
            format!("{} has struggled lately. Are you worried?", p(player))
        }
        Topic::Rumour { player, suitor } => {
            format!(
                "{} want {}. Will you sell?",
                club_name(world, suitor),
                p(player)
            )
        }
        Topic::Pressure => {
            "The board are said to be losing patience. Is your job under threat?".to_owned()
        }
        Topic::TitleRace => format!("{c} are right up there. Can you win the league?"),
        Topic::Relegation => format!("{c} are in the bottom three. Are you going down?"),
        Topic::Injury { player } => format!("How big a blow is losing {}?", p(player)),
        Topic::NewSigning { player } => format!("How is {} settling in?", p(player)),
        Topic::Youngster { player } => format!(
            "{} is getting games at {}. A star in the making?",
            p(player),
            super::refs::age(world, player)
        ),
        Topic::Unrest { player } => format!("{} wants to leave. What happens now?", p(player)),
        Topic::Snub => String::new(),
        Topic::Expectations => pick(
            salt,
            3,
            &[
                "What are your hopes for the season?",
                "What would count as a good season for you?",
                "What do the fans have to look forward to?",
            ],
        )
        .to_owned(),
        Topic::KeyPlayer { player } => {
            format!("Who's the player to watch at {c}? {} perhaps?", p(player))
        }
        Topic::AboutRival { .. } => String::new(),
        Topic::RivalComment { rival, tone, angle } => {
            let said = answer_text(world, rival, Topic::AboutRival { rival: club, angle }, tone);
            let who = rival_boss(world, rival);
            format!("{who} says: \"{said}\" Your response?")
        }
        Topic::Grudge { opponent } => format!(
            "There's no love lost between you and {}. Is this a grudge match?",
            world.clubs()[opponent]
                .manager
                .map_or_else(|| club_name(world, opponent), |m| manager_name(world, m))
        ),
    }
}

/// "Gusella's Pori Gaucci", or the club alone without a manager.
fn rival_boss(world: &World, club: ClubId) -> String {
    match world.clubs()[club].manager {
        Some(m) => format!("{}'s {}", club_name(world, club), manager_name(world, m)),
        None => club_name(world, club),
    }
}

/// What the manager says, by topic and tone.
pub fn answer_text(world: &World, club: ClubId, topic: Topic, tone: Tone) -> String {
    let p = |pl: PlayerId| player_name(world, pl);
    let c = club_name(world, club);
    use Tone::*;
    let s: String = match (topic, tone) {
        (Topic::Preview { opponent }, t) => {
            let o = club_name(world, opponent);
            match t {
                Confident => format!("We respect {o}, but we're going out to win. Simple as that."),
                Cautious => "It'll be a tough game. We take it one match at a time.".into(),
                Humble => format!("{o} are a very good side. We'll have to be at our best."),
                Dismissive => {
                    format!("Honestly? We're focused on ourselves. {o} should be worried about us.")
                }
                _ => "We'll see on the day.".into(),
            }
        }
        (Topic::RivalManager { .. }, t) => match t {
            Dismissive => "Talk is cheap. Let's see what they say after the game.".into(),
            Humble => "They're entitled to their opinion. I have a lot of respect for them.".into(),
            Defiant => "We'll let our football do the talking.".into(),
            _ => "I don't comment on other managers.".into(),
        },
        (Topic::Result { goals, .. }, t) if goals[0] > goals[1] => match t {
            Praise => "The players were magnificent. Every one of them.".into(),
            Humble => "A good result, but there's plenty to improve on.".into(),
            Confident => "This team can go all the way. I really believe that.".into(),
            _ => "Three points. That's all that matters.".into(),
        },
        (Topic::Result { goals, .. }, t) if goals[0] == goals[1] => match t {
            Praise => "I can't fault the effort. We'll take the point.".into(),
            Cautious => "A point is a point. We move on.".into(),
            Criticise => "We should have won that. It's not good enough.".into(),
            _ => "We were the better side. I'm proud of them.".into(),
        },
        (Topic::Result { .. }, t) => match t {
            Humble => "That's on me. I take full responsibility.".into(),
            Criticise => {
                "That was unacceptable. Some players need a long look in the mirror.".into()
            }
            Defiant => "We deserved more. Too many decisions went against us.".into(),
            _ => "We'll look at it on the video and move on.".into(),
        },
        (Topic::StarForm { player }, t) => match t {
            Praise => format!(
                "{} is world class. I wouldn't swap them for anyone.",
                p(player)
            ),
            Cautious => format!("{} is doing well, but there's more to come.", p(player)),
            _ => "Every player in the squad is important.".into(),
        },
        (Topic::PoorForm { player }, t) => match t {
            Praise => format!(
                "Not at all. {} has my full backing. Form is temporary.",
                p(player)
            ),
            Criticise => format!(
                "{} knows the standards here. Right now they're not meeting them.",
                p(player)
            ),
            _ => "I don't single out individuals.".into(),
        },
        (Topic::Rumour { player, .. }, t) => match t {
            Defiant => format!("{} is not for sale. End of story.", p(player)),
            Cautious => "Every player has a price. That's football.".into(),
            _ => "I don't talk about speculation.".into(),
        },
        (Topic::Pressure, t) => match t {
            Confident => "We'll turn this around. I've never been more sure of anything.".into(),
            Humble => "Results haven't been good enough. I understand the frustration.".into(),
            Defiant => "I'm not a quitter. I'll fight for this job.".into(),
            _ => "That's a question for the board.".into(),
        },
        (Topic::TitleRace, t) => match t {
            Confident => "Why not? We're good enough to win it.".into(),
            Cautious => "There's a long way to go. One game at a time.".into(),
            _ => "Ask me again in May.".into(),
        },
        (Topic::Relegation, t) => match t {
            Confident => format!("No. {c} are too good to go down."),
            Humble => "We're in a fight and we know it. Everyone has to stand up.".into(),
            _ => "Write us off at your peril.".into(),
        },
        (Topic::Injury { .. }, t) => match t {
            Confident => "It's a chance for someone else to step up. I trust the squad.".into(),
            Cautious => "We'll miss them, no doubt. We'll have to adapt.".into(),
            _ => "I don't dwell on who isn't available.".into(),
        },
        (Topic::NewSigning { player }, t) => match t {
            Praise => format!(
                "{} has been superb in training. The fans will love them.",
                p(player)
            ),
            Cautious => "It takes time to settle. We'll be patient.".into(),
            _ => "Ask me in a few months.".into(),
        },
        (Topic::Youngster { player }, t) => match t {
            Praise => format!("{} has a huge future. The sky's the limit.", p(player)),
            Cautious => "Let's not put pressure on a young player. Feet on the ground.".into(),
            _ => "We have a lot of good young players here.".into(),
        },
        (Topic::Unrest { player }, t) => match t {
            Defiant => format!("{} is under contract and going nowhere.", p(player)),
            Humble => "We'll talk privately. My door is always open.".into(),
            Criticise => format!("No player is bigger than {c}."),
            _ => "That stays in the dressing room.".into(),
        },
        (Topic::Snub, _) => String::new(),
        (Topic::Expectations, t) => match t {
            Confident => "We're here to win things. Nothing less.".into(),
            Cautious => "Improve every week and see where it takes us.".into(),
            _ => "We know where we stand. Every point will be earned.".into(),
        },
        (Topic::KeyPlayer { player }, t) => match t {
            Praise => format!("{}. Keep an eye on them, they're special.", p(player)),
            Cautious => "It's a team game. Everyone has a part to play.".into(),
            _ => "You'll have to wait and see.".into(),
        },
        // A rival manager (of `club`) on a human manager's club.
        (Topic::AboutRival { rival, angle }, t) => {
            let r = club_name(world, rival);
            match (angle, t) {
                (Angle::Reply, Dismissive) => {
                    format!("Talk is cheap. {r} should worry about their own team.")
                }
                (Angle::Reply, _) => {
                    format!("We'll do our talking on the pitch. {r} can say what they like.")
                }
                (Angle::AfterMeeting { goals }, Confident) if goals[0] > goals[1] => {
                    "We were the better side and the result shows it.".into()
                }
                (Angle::AfterMeeting { .. }, Confident) => {
                    "We'll take the point, but we came for three.".into()
                }
                (Angle::AfterMeeting { .. }, Dismissive) => {
                    format!("{r} got what they deserved. Nothing more, nothing less.")
                }
                (Angle::AfterMeeting { .. }, Defiant) => {
                    format!("We deserved more. {r} know they were lucky.")
                }
                (Angle::AfterMeeting { .. }, _) => {
                    format!("Credit to {r}. They were too good for us.")
                }
                (Angle::Race, Confident) => {
                    format!("We're in control of our own destiny. {r} can worry about us.")
                }
                (Angle::Race, _) => format!("{r} will crack. They always do when it matters."),
                (Angle::Slump, _) => format!("{r} are in freefall. I can't say I'm surprised."),
            }
        }
        (Topic::RivalComment { .. }, t) => match t {
            Dismissive => "Funny, I don't remember asking for their opinion.".into(),
            Humble => "Everyone's entitled to an opinion. I respect what they've done.".into(),
            Defiant => "Let them talk. We'll answer on the pitch.".into(),
            _ => "I'm not getting into a war of words.".into(),
        },
        (Topic::Grudge { opponent }, t) => {
            let o = club_name(world, opponent);
            match t {
                Dismissive => {
                    format!("Grudge? I don't think about {o} at all. We'll win and move on.")
                }
                Humble => {
                    "There's respect there, whatever's been said. It's just another game.".into()
                }
                _ => "It's three points like any other.".into(),
            }
        }
    };
    s
}

fn effect_view(world: &World, e: &Effect) -> Option<dto::EffectView> {
    let v = |text: String, good: Option<bool>| Some(dto::EffectView { text, good });
    match *e {
        Effect::Squad(n) if n > 0 => v("Squad morale up".into(), Some(true)),
        Effect::Squad(n) if n < 0 => v("Squad morale down".into(), Some(false)),
        Effect::Squad(_) => None,
        Effect::Opponents { club, by } if by > 0 => {
            v(format!("{} fired up", club_name(world, club)), Some(false))
        }
        Effect::Opponents { club, .. } => {
            v(format!("{} unsettled", club_name(world, club)), Some(true))
        }
        Effect::Player { player, by } if by > 0 => v(
            format!("{} pleased", player_ref(world, player).short_name),
            Some(true),
        ),
        Effect::Player { player, .. } => v(
            format!("{} upset", player_ref(world, player).short_name),
            Some(false),
        ),
        Effect::Board(n) if n > 0 => v("The board approves".into(), Some(true)),
        Effect::Board(_) => v("The board is unimpressed".into(), Some(false)),
        Effect::Rapport(n) if n > 0 => v("The press warms to you".into(), Some(true)),
        Effect::Rapport(_) => v("The press cools on you".into(), Some(false)),
        Effect::Headline(_) => v("Makes the papers".into(), None),
    }
}

fn conference_view(world: &World, c: &Conference) -> dto::ConferenceView {
    let (pre, opponent) = match c.occasion {
        Occasion::PreMatch { opponent, .. } => (true, opponent),
        Occasion::PostMatch { opponent, .. } => (false, opponent),
    };
    let questions = c
        .questions
        .iter()
        .enumerate()
        .map(|(k, q)| {
            let j = world.press().journalist(q.journalist);
            let journalist = dto::JournalistView {
                name: j.map(|j| journalist_name(world, j)).unwrap_or_default(),
                outlet: j
                    .map(|j| outlet_name(world, j.nation, j.outlet))
                    .unwrap_or_default(),
                kind: j.map_or("broadsheet", |j| outlet_kind(j.outlet)).to_owned(),
                look: journalist_look(world, q.journalist),
            };
            let option = |t: Tone| dto::AnswerOption {
                tone: tone_dto(t),
                label: tone_label(t).to_owned(),
                text: answer_text(world, c.club, q.topic, t),
            };
            dto::QuestionView {
                index: k as u8,
                journalist,
                text: question_text(world, c.club, q.topic, c.id * 8 + k as u32),
                player: q.topic.player().map(|p| player_ref(world, p)),
                options: options(q.topic).iter().map(|&t| option(t)).collect(),
                answer: q.answer.map(option),
                effects: q
                    .effects
                    .iter()
                    .filter_map(|e| effect_view(world, e))
                    .collect(),
            }
        })
        .collect();
    dto::ConferenceView {
        id: c.id,
        title: format!(
            "{} {}",
            if pre { "Before" } else { "After" },
            match c.occasion {
                Occasion::PreMatch { fixture: f, .. } | Occasion::PostMatch { fixture: f, .. } => {
                    let fx = fixture(world, f);
                    format!("{} v {}", fx.home.short_name, fx.away.short_name)
                }
            }
        ),
        pre_match: pre,
        club: club_ref(world, c.club),
        opponent: club_ref(world, opponent),
        closes: date(c.closes),
        questions,
    }
}

fn rapport_label(r: i16) -> &'static str {
    match r {
        ..=-50 => "Hostile",
        -49..=-15 => "Wary",
        -14..=14 => "Neutral",
        15..=49 => "Friendly",
        _ => "Adoring",
    }
}

/// The press room for a manager.
pub fn press(world: &World, manager: ManagerId, club: Option<ClubId>) -> dto::PressView {
    let rapport = world.press().rapport(manager);
    let quotes = world
        .news()
        .stories()
        .iter()
        .rev()
        .filter(|s| matches!(s.kind, StoryKind::Quote { manager: m, .. } if m == manager))
        .take(6)
        .map(|s| article(world, s, club))
        .collect();
    let rivalries = fs_world::rivals::rivalries(world, manager)
        .into_iter()
        .take(6)
        .map(|(rival, heat)| {
            let rival_club = world.managers()[rival].club;
            let last = club.and_then(|c| {
                world.news().stories().iter().rev().find_map(|s| match s.kind {
                    StoryKind::Quote {
                        manager: m,
                        club: from,
                        quote,
                    } if m == rival && matches!(quote.topic, Topic::AboutRival { rival: r, .. } if r == c) => {
                        Some(answer_text(world, from, quote.topic, quote.tone))
                    }
                    _ => None,
                })
            });
            dto::RivalryView {
                name: manager_name(world, rival),
                look: look(fs_world::looks::manager(world, rival)),
                club: rival_club.map(|c| club_ref(world, c)),
                heat,
                label: match heat {
                    fs_world::rivals::FEUD.. => "Feud",
                    15.. => "Needle",
                    ..=-15 => "Respect",
                    _ => "Cordial",
                }
                .into(),
                last,
            }
        })
        .collect();
    dto::PressView {
        rivalries,
        conferences: world
            .press()
            .conferences()
            .iter()
            .filter(|c| c.manager == manager)
            .map(|c| conference_view(world, c))
            .collect(),
        rapport,
        rapport_label: rapport_label(rapport).to_owned(),
        delegated: world.press().delegated(manager),
        quotes,
    }
}

/// Open conferences waiting for the manager.
pub fn waiting(world: &World, manager: ManagerId) -> u32 {
    world
        .press()
        .conferences()
        .iter()
        .filter(|c| c.manager == manager && !c.finished())
        .count() as u32
}
