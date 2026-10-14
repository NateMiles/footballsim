//! Team talks across real squads: no talk is right for everyone, and none
//! is worth more than a touchline shout on average.

use fs_gen::{GenConfig, Size, generate};
use fs_match::team_talk::{Situation, TeamTalk, reaction};
use std::collections::BTreeSet;

#[test]
fn the_right_team_talk_depends_on_the_squad() {
    let world = generate(&GenConfig::preset(Size::Small, 41));
    let ps = world.players();
    let situations = [
        Situation {
            lead: 0,
            half_time: false,
        },
        Situation {
            lead: -1,
            half_time: true,
        },
        Situation {
            lead: 0,
            half_time: true,
        },
        Situation {
            lead: 1,
            half_time: true,
        },
    ];
    for at in situations {
        let mut best = BTreeSet::new();
        let mut means = vec![0.0f64; TeamTalk::ALL.len()];
        let clubs: Vec<_> = world.clubs().ids().take(200).collect();
        for &c in &clubs {
            let squad: Vec<_> = world.clubs()[c].squad().iter().take(18).copied().collect();
            let mut scores = Vec::new();
            for (k, &talk) in TeamTalk::ALL.iter().enumerate() {
                let mean = squad
                    .iter()
                    .map(|&p| reaction(ps.hidden(p), ps.condition(p).morale, talk, at).boost())
                    .sum::<f64>()
                    / squad.len() as f64;
                means[k] += mean / clubs.len() as f64;
                scores.push((mean, k));
            }
            scores.sort_by(|a, b| b.0.total_cmp(&a.0));
            best.insert(scores[0].1);
        }
        assert!(best.len() >= 2, "{at:?}: always {best:?}, means {means:?}");
        for (k, m) in means.iter().enumerate() {
            assert!(
                (0.985..=1.02).contains(m),
                "{at:?} {:?}: {m}",
                TeamTalk::ALL[k]
            );
        }
    }
}
