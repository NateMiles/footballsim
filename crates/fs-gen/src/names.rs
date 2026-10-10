//! Procedural names built from syllables, so the game ships no real
//! people's names. Each nation gets a "style" so its names feel related.

use fs_core::Rng;

/// A family of syllables that produces names with a consistent feel.
#[derive(Debug)]
pub struct NameStyle {
    pub onsets: &'static [&'static str],
    pub vowels: &'static [&'static str],
    pub codas: &'static [&'static str],
    pub surname_endings: &'static [&'static str],
    pub town_endings: &'static [&'static str],
}

/// Built-in styles. Generated nations pick one each.
pub const STYLES: &[NameStyle] = &[
    NameStyle {
        onsets: &[
            "b", "d", "g", "l", "m", "n", "r", "s", "t", "v", "c", "f", "p",
        ],
        vowels: &["a", "e", "i", "o", "u", "ia", "io"],
        codas: &["", "", "", "n", "r", "l", "s"],
        surname_endings: &["ini", "etti", "ardo", "one", "esi", "ucci", "ano"],
        town_endings: &["ano", "ella", "ona", "ezia", "ino"],
    },
    NameStyle {
        onsets: &[
            "br", "gr", "k", "h", "w", "st", "th", "j", "m", "d", "r", "l",
        ],
        vowels: &["a", "e", "i", "o", "ea", "ou", "y"],
        codas: &["", "n", "rd", "ck", "ll", "rn", "th", "s"],
        surname_endings: &["son", "ford", "well", "ton", "ley", "wood", "er"],
        town_endings: &["ton", "bury", "field", "ham", "mouth", "wick"],
    },
    NameStyle {
        onsets: &["k", "kr", "w", "z", "b", "d", "m", "p", "s", "t", "sz", "j"],
        vowels: &["a", "e", "i", "o", "y", "ie"],
        codas: &["", "", "k", "n", "r", "sz", "ch"],
        surname_endings: &["ski", "wicz", "czak", "ek", "owski", "ak"],
        town_endings: &["ów", "ice", "awa", "no", "sk"],
    },
    NameStyle {
        onsets: &["j", "m", "r", "s", "t", "v", "g", "l", "c", "f", "b", "d"],
        vowels: &["a", "e", "i", "o", "u", "ão", "ei"],
        codas: &["", "", "s", "r", "l", "z"],
        surname_endings: &["eira", "ires", "ado", "ito", "eiro", "es", "inho"],
        town_endings: &["inha", "ada", "oso", "ela", "ores"],
    },
    NameStyle {
        onsets: &["h", "k", "s", "t", "n", "m", "y", "r", "w", "sh", "ch"],
        vowels: &["a", "i", "u", "e", "o"],
        codas: &["", "", "", "n"],
        surname_endings: &["moto", "kawa", "da", "uchi", "ta", "zaki", "no"],
        town_endings: &["ma", "saka", "hara", "zawa", "oka"],
    },
    NameStyle {
        onsets: &["v", "d", "b", "h", "k", "l", "m", "n", "r", "s", "t", "z"],
        vowels: &["a", "e", "i", "o", "u", "ee", "aa", "ij"],
        codas: &["", "n", "r", "k", "s", "rt", "ns"],
        surname_endings: &["sen", "berg", "stra", "man", "ink", "dijk", "huis"],
        town_endings: &["dam", "burg", "hoven", "rijk", "veld"],
    },
];

fn pick<'a>(rng: &mut Rng, items: &'a [&'a str]) -> &'a str {
    rng.choose(items).copied().unwrap_or("")
}

fn syllable(rng: &mut Rng, style: &NameStyle, with_coda: bool) -> String {
    let mut s = String::new();
    s.push_str(pick(rng, style.onsets));
    s.push_str(pick(rng, style.vowels));
    if with_coda {
        s.push_str(pick(rng, style.codas));
    }
    s
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

impl NameStyle {
    /// A given name of two or three syllables.
    pub fn first_name(&self, rng: &mut Rng) -> String {
        let n = 2 + rng.below(2);
        let mut s = String::new();
        for i in 0..n {
            s.push_str(&syllable(rng, self, i + 1 == n));
        }
        capitalise(&s)
    }

    /// A family name: one or two syllables plus a style ending.
    pub fn surname(&self, rng: &mut Rng) -> String {
        let n = 1 + rng.below(2);
        let mut s = String::new();
        for _ in 0..n {
            s.push_str(&syllable(rng, self, false));
        }
        s.push_str(pick(rng, self.surname_endings));
        capitalise(&s)
    }

    /// A town name.
    pub fn town(&self, rng: &mut Rng) -> String {
        let mut s = syllable(rng, self, true);
        if rng.chance(0.5) {
            s.push_str(&syllable(rng, self, false));
        }
        s.push_str(pick(rng, self.town_endings));
        capitalise(&s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_non_empty_and_capitalised() {
        let mut rng = Rng::from_u64(1);
        for style in STYLES {
            for _ in 0..50 {
                for name in [
                    style.first_name(&mut rng),
                    style.surname(&mut rng),
                    style.town(&mut rng),
                ] {
                    assert!(name.chars().count() >= 3, "{name}");
                    assert!(name.chars().next().unwrap().is_uppercase(), "{name}");
                }
            }
        }
    }
}
