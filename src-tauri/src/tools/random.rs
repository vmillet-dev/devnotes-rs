//! Passwords, drawn from the operating system's generator: never a seeded one.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::count::saturating_u32;

/// The operating system would not give randomness: nothing is drawn rather than something weak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoRandomness;

const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
/// No quote, backslash or backtick: a password pasted into a shell or a JSON string survives.
const SYMBOLS: &str = "!#$%&()*+,-./:;<=>?@[]^_{|}~";
/// What reads as something else in many fonts.
const LOOK_ALIKES: &str = "l1I0O|";

pub const MIN_LENGTH: u32 = 4;
pub const MAX_LENGTH: u32 = 256;
pub const MAX_PASSWORDS: u32 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CharacterSet {
    Lowercase,
    Uppercase,
    Digits,
    Symbols,
}

impl CharacterSet {
    fn characters(self) -> &'static str {
        match self {
            Self::Lowercase => LOWER,
            Self::Uppercase => UPPER,
            Self::Digits => DIGITS,
            Self::Symbols => SYMBOLS,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PasswordRequest {
    pub length: u32,
    /// Each chosen set is in every password drawn.
    pub sets: Vec<CharacterSet>,
    pub avoid_look_alikes: bool,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Strength {
    VeryWeak,
    Weak,
    Fair,
    Strong,
    VeryStrong,
}

impl Strength {
    /// The usual bands for a password drawn at random, not one a person chose.
    fn of(bits: f64) -> Self {
        match bits {
            bits if bits < 28.0 => Self::VeryWeak,
            bits if bits < 36.0 => Self::Weak,
            bits if bits < 60.0 => Self::Fair,
            bits if bits < 128.0 => Self::Strong,
            _ => Self::VeryStrong,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PasswordAnswer {
    Generated {
        passwords: Vec<String>,
        entropy_bits: f64,
        strength: Strength,
    },
    /// Every character set was left out.
    NoCharacters,
}

/// A uniform index below `bound`: a draw past the last whole multiple is thrown back, so no
/// character comes up more often than another.
pub(crate) fn uniform(bound: u32) -> Result<u32, NoRandomness> {
    let limit = u32::MAX - u32::MAX % bound;
    loop {
        let mut bytes = [0u8; 4];
        getrandom::fill(&mut bytes).map_err(|_| NoRandomness)?;
        let draw = u32::from_le_bytes(bytes);
        if draw < limit {
            return Ok(draw % bound);
        }
    }
}

fn draw(alphabet: &[char], length: u32) -> Result<String, NoRandomness> {
    let bound = u32::try_from(alphabet.len()).unwrap_or(u32::MAX);
    (0..length)
        .map(|_| uniform(bound).map(|index| alphabet[index as usize]))
        .collect()
}

pub fn passwords(request: &PasswordRequest) -> Result<PasswordAnswer, NoRandomness> {
    let mut chosen = request.sets.clone();
    chosen.sort_by_key(|set| *set as u8);
    chosen.dedup();
    let sets: Vec<Vec<char>> = chosen
        .into_iter()
        .map(|set| {
            set.characters()
                .chars()
                .filter(|character| {
                    !(request.avoid_look_alikes && LOOK_ALIKES.contains(*character))
                })
                .collect()
        })
        .collect();
    if sets.is_empty() {
        return Ok(PasswordAnswer::NoCharacters);
    }

    let alphabet: Vec<char> = sets.concat();
    let length = request.length.clamp(MIN_LENGTH, MAX_LENGTH);
    let passwords = (0..request.count.clamp(1, MAX_PASSWORDS))
        .map(|_| {
            loop {
                // Drawn again until every set chosen is in it: still uniform over those that qualify.
                let password = draw(&alphabet, length)?;
                if sets
                    .iter()
                    .all(|set| password.chars().any(|character| set.contains(&character)))
                {
                    break Ok(password);
                }
            }
        })
        .collect::<Result<Vec<_>, NoRandomness>>()?;

    let entropy_bits = f64::from(length) * f64::from(saturating_u32(alphabet.len())).log2();
    Ok(PasswordAnswer::Generated {
        passwords,
        strength: Strength::of(entropy_bits),
        entropy_bits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(length: u32) -> PasswordRequest {
        PasswordRequest {
            length,
            sets: vec![
                CharacterSet::Lowercase,
                CharacterSet::Uppercase,
                CharacterSet::Digits,
            ],
            avoid_look_alikes: false,
            count: 3,
        }
    }

    fn generated(answer: PasswordAnswer) -> (Vec<String>, f64, Strength) {
        match answer {
            PasswordAnswer::Generated {
                passwords,
                entropy_bits,
                strength,
            } => (passwords, entropy_bits, strength),
            PasswordAnswer::NoCharacters => panic!("no characters"),
        }
    }

    #[test]
    fn passwords_hold_every_set_chosen_and_nothing_else() {
        let (passwords, _, _) = generated(passwords(&request(12)).unwrap());

        assert_eq!(passwords.len(), 3);
        for password in &passwords {
            assert_eq!(password.chars().count(), 12);
            assert!(password.chars().any(|c| c.is_ascii_lowercase()));
            assert!(password.chars().any(|c| c.is_ascii_uppercase()));
            assert!(password.chars().any(|c| c.is_ascii_digit()));
            assert!(password.chars().all(char::is_alphanumeric));
        }
    }

    #[test]
    fn the_entropy_is_the_length_times_the_alphabet_s_bits() {
        let (_, bits, strength) = generated(passwords(&request(20)).unwrap());

        assert!((bits - 20.0 * 62f64.log2()).abs() < 1e-9);
        assert_eq!(strength, Strength::Strong);
    }

    #[test]
    fn look_alikes_are_left_out_when_asked() {
        let answer = passwords(&PasswordRequest {
            avoid_look_alikes: true,
            count: 20,
            ..request(64)
        })
        .unwrap();
        let (passwords, bits, _) = generated(answer);

        assert!(
            passwords
                .iter()
                .all(|password| !password.chars().any(|c| LOOK_ALIKES.contains(c)))
        );
        assert!(
            (bits - 64.0 * 57f64.log2()).abs() < 1e-9,
            "25 + 24 + 8 characters"
        );
    }

    #[test]
    fn nothing_is_drawn_from_no_set() {
        let answer = passwords(&PasswordRequest {
            sets: vec![],
            ..request(12)
        });

        assert_eq!(answer, Ok(PasswordAnswer::NoCharacters));
    }

    #[test]
    fn a_length_is_kept_within_its_bounds() {
        let (passwords, _, strength) = generated(
            passwords(&PasswordRequest {
                count: 1,
                ..request(1)
            })
            .unwrap(),
        );

        assert_eq!(passwords[0].len(), MIN_LENGTH as usize);
        assert_eq!(strength, Strength::VeryWeak);
    }
}
