//! Note-name and drum-key helpers — the musician-friendly entry points.
//!
//! The Roland MC family follows the **Roland / Yamaha** octave convention:
//! middle C is **C3 = MIDI 60**, so a kick on C1 is MIDI 36. (The "scientific
//! pitch notation" used by some software puts middle C at C4 = 60, which maps
//! the same kick to C2. We follow Roland here so the numbers match what shows
//! on the MC-707 / MC-101 screen.)
//!
//! Quick reference for the most common drum keys:
//!
//! | name | MIDI | drum |
//! |------|------|------|
//! | `C1` | 36   | kick |
//! | `D1` | 38   | snare |
//! | `F#1`| 42   | closed hat |
//! | `A#1`| 46   | open hat |

use crate::error::{Error, Result};

// --- Drum-kit shortcuts (General MIDI / Roland defaults) -------------------
pub const KICK: u8 = 36; // C1
pub const SIDE_STICK: u8 = 37; // C#1
pub const SNARE: u8 = 38; // D1
pub const CLAP: u8 = 39; // D#1
pub const SNARE_RIM: u8 = 40; // E1
pub const TOM_LOW: u8 = 41; // F1
pub const HAT_CLOSED: u8 = 42; // F#1
pub const TOM_LOW_MID: u8 = 43; // G1
pub const HAT_PEDAL: u8 = 44; // G#1
pub const TOM_MID: u8 = 45; // A1
pub const HAT_OPEN: u8 = 46; // A#1
pub const TOM_HIGH_MID: u8 = 47; // B1
pub const TOM_HIGH: u8 = 48; // C2
pub const CRASH: u8 = 49; // C#2
pub const RIDE: u8 = 51; // D#2

fn note_offset(letter: &str) -> Option<i32> {
    Some(match letter {
        "C" => 0,
        "C#" => 1,
        "DB" => 1,
        "D" => 2,
        "D#" => 3,
        "EB" => 3,
        "E" => 4,
        "F" => 5,
        "F#" => 6,
        "GB" => 6,
        "G" => 7,
        "G#" => 8,
        "AB" => 8,
        "A" => 9,
        "A#" => 10,
        "BB" => 10,
        "B" => 11,
        _ => return None,
    })
}

/// Convert a note name like `"C1"` to a MIDI note number (0..127).
///
/// Accepts:
///
/// * names with an explicit octave — `"C1"` → 36, `"F#3"` → 66
/// * names without an octave (assumes octave 3, i.e. middle octave) —
///   `"C"` → 60
/// * lower or mixed case — `"eb2"` → 51
/// * negative octaves — `"C-2"` → 0 (Roland puts the lowest note at C-2)
/// * both `#` and `b` accidentals — `"Bb1"` → 46
///
/// Plain MIDI numbers are handled directly by [`Pitch`]; wrap an existing
/// number with [`Pitch::from`] and it passes through unchanged.
pub fn note(name: &str) -> Result<u8> {
    let s = name.trim();
    if s.is_empty() {
        return Err(Error::Value("note name is empty".to_string()));
    }
    if !s.is_ascii() {
        return Err(Error::Value(format!(
            "unrecognised note name {name:?} — note names must be ASCII"
        )));
    }

    let upper = s.to_uppercase();
    let chars: Vec<char> = upper.chars().collect();

    // Pull off the letter + optional accidental (Python: second char in "#B").
    let (letter, rest): (&str, &str) = if chars.len() >= 2 && (chars[1] == '#' || chars[1] == 'B') {
        (&upper[0..2], &upper[2..])
    } else {
        (&upper[0..1], &upper[1..])
    };

    let offset = note_offset(letter).ok_or_else(|| {
        Error::Value(format!(
            "unrecognised note name {name:?} — expected something like \
             'C1', 'F#3', 'Bb-1', or a MIDI number 0..127"
        ))
    })?;

    let octave: i32 = if rest.is_empty() {
        3 // Roland convention: C3 is middle C
    } else {
        rest.parse().map_err(|_| {
            Error::Value(format!(
                "unrecognised note name {name:?} — couldn't parse octave {rest:?}"
            ))
        })?
    };

    let midi = (octave + 2) * 12 + offset;
    if !(0..=127).contains(&midi) {
        return Err(Error::Value(format!(
            "note {name:?} resolves to MIDI {midi} which is out of range 0..127"
        )));
    }
    Ok(midi as u8)
}

/// Inverse of [`note`] — turn a MIDI number back into a name like `"C3"`.
///
/// Returns the sharp form for accidentals (`"C#1"`, never `"Db1"`).
pub fn note_name(midi: u8) -> Result<String> {
    if midi > 127 {
        return Err(Error::Value(format!("MIDI note out of range: {midi}")));
    }
    const SHARPS: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = (midi as i32) / 12 - 2; // Roland convention (C3 = 60)
    Ok(format!("{}{}", SHARPS[(midi % 12) as usize], octave))
}

/// Accept either a MIDI number or a note name and resolve it to 0..127.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pitch {
    Midi(u8),
    Note(String),
}

impl Pitch {
    pub fn resolve(&self) -> Result<u8> {
        match self {
            Pitch::Midi(n) => {
                if *n <= 127 {
                    Ok(*n)
                } else {
                    Err(Error::Value(format!(
                        "MIDI note out of range: {n} (must be 0..127)"
                    )))
                }
            }
            Pitch::Note(s) => note(s),
        }
    }
}

impl From<u8> for Pitch {
    fn from(n: u8) -> Self {
        Pitch::Midi(n)
    }
}

impl From<&str> for Pitch {
    fn from(s: &str) -> Self {
        Pitch::Note(s.to_string())
    }
}

impl From<String> for Pitch {
    fn from(s: String) -> Self {
        Pitch::Note(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drum_keys() {
        assert_eq!(KICK, 36);
        assert_eq!(note("C1").unwrap(), 36);
        assert_eq!(note("D1").unwrap(), 38);
        assert_eq!(note("F#1").unwrap(), 42);
        assert_eq!(note("A#1").unwrap(), 46);
    }

    #[test]
    fn parses_accidentals_and_case() {
        assert_eq!(note("C3").unwrap(), 60);
        assert_eq!(note("eb2").unwrap(), 51);
        assert_eq!(note("Bb1").unwrap(), 46);
        assert_eq!(note("Bb-1").unwrap(), 22);
    }

    #[test]
    fn negative_octaves() {
        // Roland convention: C3 = 60, so C-2 is the lowest note (0).
        assert_eq!(note("C-1").unwrap(), 12);
        assert_eq!(note("C-2").unwrap(), 0);
    }

    #[test]
    fn default_octave_is_middle() {
        assert_eq!(note("C").unwrap(), 60);
    }

    #[test]
    fn name_round_trips_all_midi() {
        for n in 0..=127u8 {
            let name = note_name(n).unwrap();
            assert_eq!(note(&name).unwrap(), n);
        }
    }

    #[test]
    fn rejects_out_of_range() {
        assert!(note("C9").is_err());
        assert!(note("C-3").is_err());
        assert!(note("Hzz").is_err());
    }

    #[test]
    fn pitch_resolves() {
        assert_eq!(Pitch::from(60).resolve().unwrap(), 60);
        assert_eq!(Pitch::from("C1").resolve().unwrap(), 36);
    }
}
