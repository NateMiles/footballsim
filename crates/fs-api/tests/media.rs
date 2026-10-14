//! The news and the press room through the protocol.

use fs_api::dto::{Art, NewsScope, Tone, WorldSize};
use fs_api::{ErrorCode, Request, Response, Session};

fn call(s: &mut Session, r: Request) -> Response {
    let json = serde_json::to_string(&r).unwrap();
    serde_json::from_str(&s.call_json(&json)).unwrap()
}

macro_rules! expect {
    ($response:expr, $variant:ident { $field:ident }) => {
        match $response {
            Response::$variant { $field } => $field,
            other => panic!("expected {}, got {other:?}", stringify!($variant)),
        }
    };
}

fn career() -> Session {
    let mut s = Session::new();
    let choices = expect!(
        call(
            &mut s,
            Request::NewWorld {
                seed: 33,
                size: WorldSize::Tiny
            }
        ),
        ClubChoices { choices }
    );
    let club = choices.clubs[6].club.id;
    expect!(
        call(
            &mut s,
            Request::StartCareer {
                club,
                first_name: "Sam".into(),
                last_name: "Reed".into(),
            }
        ),
        Overview { overview }
    );
    expect!(
        call(&mut s, Request::SetWatchMatches { watch: false }),
        Overview { overview }
    );
    s
}

/// Continues until a press conference is waiting.
fn until_press(s: &mut Session) {
    for _ in 0..30 {
        let o = expect!(call(s, Request::Overview), Overview { overview });
        if o.press_waiting > 0 {
            return;
        }
        expect!(call(s, Request::Continue), Continued { continued });
    }
    panic!("no press conference");
}

#[test]
fn press_conferences_are_answered_through_the_protocol() {
    let mut s = career();
    until_press(&mut s);
    let inbox = expect!(call(&mut s, Request::Inbox), Inbox { inbox });
    assert!(
        inbox
            .messages
            .iter()
            .any(|m| m.title.starts_with("Press conference"))
    );
    let press = expect!(call(&mut s, Request::Press), Press { press });
    assert_eq!(press.rapport_label, "Neutral");
    let c = &press.conferences[0];
    assert!(c.pre_match);
    let q = &c.questions[0];
    assert!(!q.text.is_empty() && !q.journalist.name.is_empty());
    assert!(q.options.len() >= 3);
    assert!(q.options.iter().all(|o| !o.text.is_empty()));

    let confident = q
        .options
        .iter()
        .find(|o| o.tone == Tone::Confident)
        .unwrap()
        .tone;
    let press = expect!(
        call(
            &mut s,
            Request::AnswerQuestion {
                conference: c.id,
                question: 0,
                tone: confident,
            }
        ),
        Press { press }
    );
    let q = &press.conferences[0].questions[0];
    assert_eq!(q.answer.as_ref().unwrap().tone, Tone::Confident);
    assert!(q.effects.iter().any(|e| e.text == "Squad morale up"));
    assert!(q.effects.iter().any(|e| e.text == "Makes the papers"));
    assert_eq!(press.quotes.len(), 1);

    // Answering twice is refused.
    match call(
        &mut s,
        Request::AnswerQuestion {
            conference: c.id,
            question: 0,
            tone: confident,
        },
    ) {
        Response::Error { error } => assert_eq!(error.code, ErrorCode::Rejected),
        other => panic!("{other:?}"),
    }

    // The quote is in the club's news, with a picture of the manager.
    let news = expect!(
        call(
            &mut s,
            Request::News {
                scope: NewsScope::Club
            }
        ),
        News { news }
    );
    let quote = news
        .articles
        .iter()
        .find(|a| matches!(a.art, Art::Microphone { .. }))
        .unwrap();
    assert!(quote.mine);
    assert!(quote.body[0].contains("We respect"));

    // The rest goes to the assistant.
    let press = expect!(
        call(&mut s, Request::LeaveConference { conference: c.id }),
        Press { press }
    );
    assert!(press.conferences.is_empty());
    let press = expect!(
        call(&mut s, Request::SetPressDelegation { delegate: true }),
        Press { press }
    );
    assert!(press.delegated);
}

#[test]
fn the_news_has_something_for_every_scope() {
    let mut s = career();
    for _ in 0..8 {
        expect!(call(&mut s, Request::Continue), Continued { continued });
    }
    let o = expect!(call(&mut s, Request::Overview), Overview { overview });
    assert!(!o.headlines.is_empty());
    for scope in [NewsScope::Club, NewsScope::Nation, NewsScope::World] {
        let news = expect!(call(&mut s, Request::News { scope }), News { news });
        assert!(!news.articles.is_empty(), "{scope:?}");
        for a in &news.articles {
            assert!(!a.headline.is_empty() && !a.standfirst.is_empty());
            assert!(!a.byline.outlet.is_empty());
        }
        assert!(news.articles.windows(2).all(|w| w[0].date >= w[1].date));
    }
}

#[test]
fn a_dismissive_word_starts_a_rivalry() {
    let mut s = career();
    until_press(&mut s);
    let press = expect!(call(&mut s, Request::Press), Press { press });
    assert!(press.rivalries.is_empty());
    let c = &press.conferences[0];
    let press = expect!(
        call(
            &mut s,
            Request::AnswerQuestion {
                conference: c.id,
                question: 0,
                tone: Tone::Dismissive,
            }
        ),
        Press { press }
    );
    let r = &press.rivalries[0];
    assert_eq!(r.label, "Needle");
    assert_eq!(r.heat, 15);
    assert_eq!(r.club.as_ref().unwrap().id, c.opponent.id);
}
