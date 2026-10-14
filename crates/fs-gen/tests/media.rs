//! The news and the press: stories filed from around the world, press
//! conferences before and after matches, and what answers do.

use fs_gen::{GenConfig, Size, generate};
use fs_world::news::StoryKind;
use fs_world::press::{Effect, Occasion, Tone, options};
use fs_world::{Actor, ClubId, Command, CommandError, Event, Game, ManagerId};

fn one_human(size: Size, seed: u64, club: u32) -> (Game, ManagerId, ClubId) {
    let mut game = Game::new(generate(&GenConfig {
        human_clubs: vec![club],
        ..GenConfig::preset(size, seed)
    }));
    let m = game.world().human_managers().next().unwrap();
    game.submit(Actor::Manager(m), Command::SetDelegation { squad: true })
        .unwrap();
    (game, m, ClubId::new(club))
}

fn day(game: &mut Game) -> Vec<Event> {
    game.submit(Actor::System, Command::Advance { days: 1 })
        .unwrap()
}

/// Advances a day at a time until a press conference opens.
fn until_conference(game: &mut Game) -> u32 {
    for _ in 0..120 {
        for e in day(game) {
            if let Event::PressConference { conference, .. } = e {
                return conference;
            }
        }
    }
    panic!("no press conference in 120 days");
}

#[test]
fn the_news_covers_the_world_without_drowning_in_it() {
    let (mut game, _, club) = one_human(Size::Small, 21, 0);
    let count = |game: &Game, f: fn(&StoryKind) -> bool| {
        game.world()
            .news()
            .stories()
            .iter()
            .filter(|s| f(&s.kind))
            .count()
    };
    // The summer window: deals and rumours.
    game.submit(Actor::System, Command::Advance { days: 75 })
        .unwrap();
    assert!(count(&game, |k| matches!(k, StoryKind::Transfer { .. })) > 5);
    assert!(count(&game, |k| matches!(k, StoryKind::Rumour { .. })) > 5);
    game.submit(Actor::System, Command::Advance { days: 125 })
        .unwrap();
    let stories = game.world().news().stories();
    let mut by: std::collections::BTreeMap<String, usize> = Default::default();
    for s in stories {
        let k = format!("{:?}", s.kind);
        *by.entry(k.split([' ', '{']).next().unwrap().to_owned())
            .or_default() += 1;
    }
    // Small world: about 120 leagues. A few hundred stories in four
    // months is a busy paper, not a phone book.
    assert!((100..1_200).contains(&stories.len()), "{by:?}");
    assert!(count(&game, |k| matches!(k, StoryKind::Result { .. })) > 20);
    // Everything about the human club is kept.
    let ours = stories
        .iter()
        .filter(|s| fs_world::news::clubs_of(&s.kind).contains(&Some(club)))
        .count();
    assert!(ours >= 10, "{ours} stories about the human club");
    // Ids are in filing order, and nothing older than the cut-off stays.
    assert!(stories.windows(2).all(|w| w[0].id < w[1].id));
    let today = game.world().date();
    assert!(
        stories
            .iter()
            .all(|s| today.days_since(s.date) <= fs_world::news::KEEP_DAYS)
    );
}

#[test]
fn press_conferences_ask_about_the_club_and_answers_have_effects() {
    let (mut game, m, club) = one_human(Size::Tiny, 8, 2);
    let me = Actor::Manager(m);
    let id = until_conference(&mut game);
    let c = game.world().press().conference(id).unwrap().clone();
    assert_eq!(c.club, club);
    assert!(matches!(c.occasion, Occasion::PreMatch { .. }));
    assert!(c.questions.len() >= 2);

    // An answer that is not on offer is refused.
    let q0 = c.questions[0].topic;
    let wrong = [
        Tone::Confident,
        Tone::Cautious,
        Tone::Praise,
        Tone::Criticise,
        Tone::Deflect,
        Tone::Defiant,
        Tone::Humble,
        Tone::Dismissive,
    ]
    .into_iter()
    .find(|t| !options(q0).contains(t))
    .unwrap();
    assert!(matches!(
        game.submit(
            me,
            Command::AnswerQuestion {
                conference: id,
                question: 0,
                tone: wrong
            }
        ),
        Err(CommandError::UnsuitableAnswer(_))
    ));

    // Confident about the game lifts the squad and is quoted.
    let morale = |game: &Game| -> u64 {
        let w = game.world();
        w.clubs()[club]
            .squad()
            .iter()
            .map(|&p| u64::from(w.players().condition(p).morale))
            .sum()
    };
    let before = morale(&game);
    game.submit(
        me,
        Command::AnswerQuestion {
            conference: id,
            question: 0,
            tone: Tone::Confident,
        },
    )
    .unwrap();
    assert!(morale(&game) > before);
    let q = &game.world().press().conference(id).unwrap().questions[0];
    assert_eq!(q.answer, Some(Tone::Confident));
    let headline = q.effects.iter().find_map(|e| match e {
        Effect::Headline(s) => Some(*s),
        _ => None,
    });
    let story = game.world().news().get(headline.unwrap()).unwrap();
    assert!(matches!(story.kind, StoryKind::Quote { club: c, .. } if c == club));

    // A question is answered once.
    assert!(matches!(
        game.submit(
            me,
            Command::AnswerQuestion {
                conference: id,
                question: 0,
                tone: Tone::Cautious
            }
        ),
        Err(CommandError::AlreadyAnswered(_))
    ));

    // Every other question takes each of its answers.
    for (k, q) in c.questions.iter().enumerate().skip(1) {
        let tone = options(q.topic)[0];
        game.submit(
            me,
            Command::AnswerQuestion {
                conference: id,
                question: k as u8,
                tone,
            },
        )
        .unwrap();
    }
    assert!(game.world().press().conference(id).unwrap().finished());

    // After the match comes the post-match conference, about the result.
    let post = until_conference(&mut game);
    let c = game.world().press().conference(post).unwrap();
    assert!(matches!(c.occasion, Occasion::PostMatch { .. }));
    assert!(
        c.questions
            .iter()
            .any(|q| matches!(q.topic, fs_world::press::Topic::Result { .. }))
    );
}

#[test]
fn ignored_conferences_are_noticed_and_delegated_ones_are_not_held() {
    let (mut game, m, club) = one_human(Size::Tiny, 9, 4);
    let id = until_conference(&mut game);
    // Let it close unattended.
    while game.world().press().conference(id).is_some() {
        day(&mut game);
    }
    assert!(game.world().press().rapport(m) < 0);
    assert!(game.world().news().stories().iter().any(|s| matches!(
        s.kind,
        StoryKind::Quote { club: c, quote, .. }
            if c == club && quote.topic == fs_world::press::Topic::Snub
    )));

    game.submit(
        Actor::Manager(m),
        Command::SetPressDelegation { delegate: true },
    )
    .unwrap();
    let until = game.world().date().add_days(60);
    while game.world().date() < until {
        for e in day(&mut game) {
            assert!(!matches!(e, Event::PressConference { .. }));
        }
    }
    assert!(game.world().press().conferences().is_empty());
}

#[test]
fn rival_managers_answer_back_and_feuds_grow() {
    let (mut game, m, club) = one_human(Size::Tiny, 12, 3);
    let me = Actor::Manager(m);
    let mut spoke = 0;
    let mut asked_back = false;
    for _ in 0..90 {
        for e in day(&mut game) {
            match e {
                Event::RivalSpoke {
                    manager, club: c, ..
                } => {
                    assert_eq!((manager, c), (m, club));
                    spoke += 1;
                }
                Event::PressConference { conference, .. } => {
                    // Wave every opponent away.
                    let c = game.world().press().conference(conference).unwrap().clone();
                    for (k, q) in c.questions.iter().enumerate() {
                        if matches!(q.topic, fs_world::press::Topic::RivalComment { .. }) {
                            asked_back = true;
                        }
                        let tone = if options(q.topic).contains(&Tone::Dismissive) {
                            Tone::Dismissive
                        } else {
                            options(q.topic)[0]
                        };
                        game.submit(
                            me,
                            Command::AnswerQuestion {
                                conference,
                                question: k as u8,
                                tone,
                            },
                        )
                        .unwrap();
                    }
                }
                _ => {}
            }
        }
    }
    assert!(spoke >= 2, "{spoke} rival comments");
    assert!(asked_back, "the press never asked about a rival's words");
    let rivals = fs_world::rivals::rivalries(game.world(), m);
    assert!(rivals.first().is_some_and(|&(_, h)| h > 0), "{rivals:?}");
    // Rivals' words are in the news, about this club.
    assert!(game.world().news().stories().iter().any(|s| matches!(
        s.kind,
        StoryKind::Quote { quote, .. }
            if matches!(quote.topic, fs_world::press::Topic::AboutRival { rival, .. } if rival == club)
    )));
}
