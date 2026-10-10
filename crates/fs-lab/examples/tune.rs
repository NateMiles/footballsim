//! Tier 1 tuning harness. Overrides engine constants from `key=value`
//! arguments, simulates eight seasons and prints the calibration checks
//! plus extra per-team averages.
//!
//! ```text
//! cargo run --release -p fs-lab --example tune -- slope=3.0 foul=0.012
//! ```
use fs_lab::{Engine, report, simulate_seasons_with};
use fs_match::tier1::Params;

fn main() {
    let mut p = Params::default();
    let args: Vec<String> = std::env::args().skip(1).collect();
    for a in &args {
        let (k, v) = a.split_once('=').expect("key=value");
        let v: f64 = v.parse().expect("number");
        match k {
            "slope" => p.skill_slope = v,
            "home" => p.home_logit = v,
            "pslope" => p.pass_slope = v,
            "direct" => p.direct_effect = v,
            "press" => p.press_effect = v,
            "fwd" => p.forward_rate = v,
            "through" => p.through_base = v,
            "cross" => p.cross_base = v,
            "dribble" => p.dribble_base = v,
            "boxw" => p.box_shot_weight = v,
            "foul" => p.foul_rate = v,
            "card" => p.card_rate = v,
            "red" => p.straight_red = v,
            "big" => p.xg_big_chance = v,
            "header" => p.xg_header = v,
            "ontarget" => p.on_target_base = v,
            "s4" => p.short_base[4] = v,
            "s5" => p.short_base[5] = v,
            "s3" => p.short_base[3] = v,
            "s0" => p.short_base[0] = v,
            "s1" => p.short_base[1] = v,
            "s2" => p.short_base[2] = v,
            "sall" => {
                for x in p.short_base.iter_mut() {
                    *x += v;
                }
            }
            "xc5" => p.xg_central[5] = v,
            "xw5" => p.xg_wide[5] = v,
            "keeper" => p.average_keeper = v,
            "xc4" => p.xg_central[4] = v,
            "corner" => p.xg_corner_header = v,
            "secs" => p.pass_secs = v,
            "long" => p.long_base = v,
            "expose" => p.exposure_effect = v,
            "frisk" => p.forward_risk = v,
            "chase" => p.chase_fatigue = v,
            "drain" => p.energy_drain = v,
            _ => panic!("unknown {k}"),
        }
    }
    let seasons = simulate_seasons_with(1, 16, &Engine::Event(p));
    {
        // Average keeper skill, to set average_keeper.
        let w = fs_lab::league_world(1);
        let mut k = 0.0;
        for c in 0..20u32 {
            let sheet = fs_world::matchday::team_sheet(&w, fs_world::ClubId::new(c));
            k += fs_match::skills::Skills::of(
                &sheet.starters[0],
                fs_core::attributes::Position::Goalkeeper,
            )
            .keeper;
        }
        println!("{:<30} {:>8.3}", "mean keeper skill", k / 20.0);
    }
    let r = report(&seasons);
    for c in &r.checks {
        println!(
            "{:<30} {:>8.3}  [{} .. {}] {}",
            c.name,
            c.value,
            c.low,
            c.high,
            if c.ok() { "" } else { "<<" }
        );
    }
    let mut t = [0.0f64; 8];
    let mut n = 0.0;
    for s in &seasons {
        for m in &s.matches {
            for st in &m.result.stats {
                n += 1.0;
                t[0] += f64::from(st.crosses);
                t[1] += f64::from(st.crosses_completed);
                t[2] += f64::from(st.dribbles);
                t[3] += f64::from(st.dribbles_won);
                t[4] += f64::from(st.tackles);
                t[5] += f64::from(st.interceptions);
                t[6] += f64::from(st.offsides);
                t[7] += f64::from(st.saves);
            }
        }
    }
    let names = [
        "crosses",
        "crosses won",
        "dribbles",
        "dribbles won",
        "tackles",
        "interceptions",
        "offsides",
        "saves",
    ];
    for (k, v) in names.iter().zip(t) {
        println!("{k:<30} {:>8.2}", v / n);
    }
}
