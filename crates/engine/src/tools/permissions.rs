//! A Unix mode read from its octal digits or from the string `ls -l` prints, written back in
//! both, with the `chmod` commands that set it and what a umask leaves a new file.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::count::saturating_u32;

const SETUID: u32 = 0o4000;
const SETGID: u32 = 0o2000;
const STICKY: u32 = 0o1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FileType {
    Regular,
    Directory,
    SymbolicLink,
    CharacterDevice,
    BlockDevice,
    NamedPipe,
    Socket,
}

impl FileType {
    fn read(letter: char) -> Option<Self> {
        Some(match letter {
            '-' => Self::Regular,
            'd' => Self::Directory,
            'l' => Self::SymbolicLink,
            'c' => Self::CharacterDevice,
            'b' => Self::BlockDevice,
            'p' => Self::NamedPipe,
            's' => Self::Socket,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Class {
    Owner,
    Group,
    Others,
}

const CLASSES: [Class; 3] = [Class::Owner, Class::Group, Class::Others];

impl Class {
    fn shift(self) -> u32 {
        match self {
            Self::Owner => 6,
            Self::Group => 3,
            Self::Others => 0,
        }
    }

    /// The bit that runs as this class, or sticks: setuid, setgid, sticky.
    fn special(self) -> u32 {
        match self {
            Self::Owner => SETUID,
            Self::Group => SETGID,
            Self::Others => STICKY,
        }
    }

    fn letter(self) -> char {
        match self {
            Self::Owner => 'u',
            Self::Group => 'g',
            Self::Others => 'o',
        }
    }
}

/// A row of the grid: its four boxes are four booleans, not a state machine.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClassRights {
    pub class: Class,
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    /// Setuid for the owner, setgid for the group, the sticky bit for the others.
    pub special: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Mode {
    /// The twelve bits, `0o7777` at most.
    pub bits: u32,
    /// Three digits, or four when a special bit is set: `755`, `4755`.
    pub octal: String,
    pub symbolic: String,
    /// `u=rwx,g=rx,o=rx`, what `chmod` takes to set exactly this mode.
    pub chmod_symbolic: String,
    pub classes: Vec<ClassRights>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ModeProblem {
    /// Neither octal digits nor a mode string.
    Unreadable,
    /// A digit past 7.
    NotOctal,
    /// More than 0o7777.
    TooLarge,
    /// A letter where this position of a mode string takes another.
    Unexpected,
    /// A mode string is nine letters, ten with the file type in front.
    Length,
}

/// A mode that is probably a mistake: legal, but rarely what was meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ModeWarning {
    /// The group holds less than everybody: the others have a right it lacks.
    OthersOverGroup,
    /// The owner holds less than its group.
    GroupOverOwner,
    /// Anyone may write, and no sticky bit keeps each to their own files.
    WorldWritable,
    /// Setuid or setgid without the execute right it acts on: `S` in `ls -l`.
    SpecialWithoutExecute,
}

fn warnings(bits: u32) -> Vec<ModeWarning> {
    let rights = |class: Class| (bits >> class.shift()) & 0o7;
    let (owner, group, others) = (
        rights(Class::Owner),
        rights(Class::Group),
        rights(Class::Others),
    );
    [
        (others & !group != 0, ModeWarning::OthersOverGroup),
        (group & !owner != 0, ModeWarning::GroupOverOwner),
        (
            others & 2 != 0 && bits & STICKY == 0,
            ModeWarning::WorldWritable,
        ),
        (
            (bits & SETUID != 0 && owner & 1 == 0) || (bits & SETGID != 0 && group & 1 == 0),
            ModeWarning::SpecialWithoutExecute,
        ),
    ]
    .into_iter()
    .filter_map(|(holds, warning)| holds.then_some(warning))
    .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ModeReading {
    Empty,
    Read {
        mode: Mode,
        /// Named by the letter a mode string of `ls -l` starts with.
        file_type: Option<FileType>,
        warnings: Vec<ModeWarning>,
    },
    Refused {
        problem: ModeProblem,
        /// One-based, in characters.
        at: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum UmaskReading {
    Empty,
    Read {
        file: Box<Mode>,
        directory: Box<Mode>,
    },
    Refused {
        problem: ModeProblem,
        at: u32,
    },
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsRequest {
    /// `755`, `0644`, `4755`, `rwxr-xr-x`, `drwxr-xr-x`.
    pub mode: String,
    pub umask: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsAnswer {
    pub mode: ModeReading,
    pub umask: UmaskReading,
}

/// A refusal and its one-based character.
type Located = (ModeProblem, u32);

fn refused(problem: ModeProblem, at: usize) -> Located {
    (problem, saturating_u32(at) + 1)
}

fn describe_mode(bits: u32) -> Mode {
    let symbolic: String = CLASSES
        .iter()
        .flat_map(|class| {
            let rights = bits >> class.shift();
            let execute = rights & 1 != 0;
            let special = bits & class.special() != 0;
            let last = match (special, execute, class) {
                (true, true, Class::Others) => 't',
                (true, false, Class::Others) => 'T',
                (true, true, _) => 's',
                (true, false, _) => 'S',
                (false, true, _) => 'x',
                (false, false, _) => '-',
            };
            [
                if rights & 4 != 0 { 'r' } else { '-' },
                if rights & 2 != 0 { 'w' } else { '-' },
                last,
            ]
        })
        .collect();

    let chmod_symbolic = CLASSES
        .iter()
        .map(|class| {
            let rights = bits >> class.shift();
            let mut letters = String::new();
            for (bit, letter) in [(4, 'r'), (2, 'w'), (1, 'x')] {
                if rights & bit != 0 {
                    letters.push(letter);
                }
            }
            if bits & class.special() != 0 {
                letters.push(if *class == Class::Others { 't' } else { 's' });
            }
            format!("{}={letters}", class.letter())
        })
        .collect::<Vec<_>>()
        .join(",");

    Mode {
        bits,
        octal: if bits > 0o777 {
            format!("{bits:04o}")
        } else {
            format!("{bits:03o}")
        },
        symbolic,
        chmod_symbolic,
        classes: CLASSES
            .iter()
            .map(|class| {
                let rights = bits >> class.shift();
                ClassRights {
                    class: *class,
                    read: rights & 4 != 0,
                    write: rights & 2 != 0,
                    execute: rights & 1 != 0,
                    special: bits & class.special() != 0,
                }
            })
            .collect(),
    }
}

fn octal(text: &str, skipped: usize) -> Result<u32, Located> {
    let digits = text
        .strip_prefix("0o")
        .or_else(|| text.strip_prefix("0O"))
        .unwrap_or(text);
    let lead = skipped + text.len() - digits.len();
    let mut bits = 0u32;
    for (index, digit) in digits.chars().enumerate() {
        match digit.to_digit(8) {
            Some(value) => bits = bits.saturating_mul(8).saturating_add(value),
            None if digit.is_ascii_digit() => {
                return Err(refused(ModeProblem::NotOctal, lead + index));
            }
            None => return Err(refused(ModeProblem::Unreadable, lead + index)),
        }
    }
    if bits > 0o7777 {
        return Err(refused(ModeProblem::TooLarge, lead));
    }
    Ok(bits)
}

/// Nine letters, the file type's in front of them or not, and `ls -l`'s `.`, `+` or `@` after.
fn symbolic(text: &str, skipped: usize) -> Result<(u32, Option<FileType>), Located> {
    let mut letters: Vec<char> = text.chars().collect();
    if matches!(letters.last(), Some('.' | '+' | '@')) && letters.len() == 11 {
        letters.pop();
    }
    let (file_type, rights, lead) = match letters.len() {
        10 => match FileType::read(letters[0]) {
            Some(file_type) => (Some(file_type), &letters[1..], skipped + 1),
            None => return Err(refused(ModeProblem::Unexpected, skipped)),
        },
        9 => (None, &letters[..], skipped),
        _ => return Err(refused(ModeProblem::Length, skipped)),
    };

    let mut bits = 0u32;
    for (class_index, class) in CLASSES.iter().enumerate() {
        let at = |offset: usize| lead + class_index * 3 + offset;
        let triple = &rights[class_index * 3..class_index * 3 + 3];
        let shift = class.shift();
        match triple[0] {
            'r' => bits |= 4 << shift,
            '-' => {}
            _ => return Err(refused(ModeProblem::Unexpected, at(0))),
        }
        match triple[1] {
            'w' => bits |= 2 << shift,
            '-' => {}
            _ => return Err(refused(ModeProblem::Unexpected, at(1))),
        }
        let (special, lower, upper) = if *class == Class::Others {
            (STICKY, 't', 'T')
        } else {
            (class.special(), 's', 'S')
        };
        match triple[2] {
            'x' => bits |= 1 << shift,
            '-' => {}
            letter if letter == lower => bits |= special | (1 << shift),
            letter if letter == upper => bits |= special,
            _ => return Err(refused(ModeProblem::Unexpected, at(2))),
        }
    }
    Ok((bits, file_type))
}

fn read_mode(text: &str) -> Result<Option<(u32, Option<FileType>)>, Located> {
    let trimmed = text.trim_start();
    let skipped = text.chars().count() - trimmed.chars().count();
    let trimmed = trimmed.trim_end();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.starts_with(|c: char| c.is_ascii_digit()) {
        octal(trimmed, skipped).map(|bits| Some((bits, None)))
    } else {
        symbolic(trimmed, skipped).map(Some)
    }
}

pub fn describe(request: &PermissionsRequest) -> PermissionsAnswer {
    let mode = match read_mode(&request.mode) {
        Ok(None) => ModeReading::Empty,
        Ok(Some((bits, file_type))) => ModeReading::Read {
            mode: describe_mode(bits),
            file_type,
            warnings: warnings(bits),
        },
        Err((problem, at)) => ModeReading::Refused { problem, at },
    };
    let umask = match read_mode(&request.umask) {
        Ok(None) => UmaskReading::Empty,
        Ok(Some((mask, _))) => UmaskReading::Read {
            file: Box::new(describe_mode(0o666 & !mask)),
            directory: Box::new(describe_mode(0o777 & !mask)),
        },
        Err((problem, at)) => UmaskReading::Refused { problem, at },
    };
    PermissionsAnswer { mode, umask }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(text: &str) -> (Mode, Option<FileType>) {
        match describe(&PermissionsRequest {
            mode: text.to_owned(),
            umask: String::new(),
        })
        .mode
        {
            ModeReading::Read {
                mode, file_type, ..
            } => (mode, file_type),
            other => panic!("{text} was not read: {other:?}"),
        }
    }

    fn refusal(text: &str) -> (ModeProblem, u32) {
        match describe(&PermissionsRequest {
            mode: text.to_owned(),
            umask: String::new(),
        })
        .mode
        {
            ModeReading::Refused { problem, at } => (problem, at),
            other => panic!("{text} was not refused: {other:?}"),
        }
    }

    fn warned(text: &str) -> Vec<ModeWarning> {
        match describe(&PermissionsRequest {
            mode: text.to_owned(),
            umask: String::new(),
        })
        .mode
        {
            ModeReading::Read { warnings, .. } => warnings,
            other => panic!("{text} was not read: {other:?}"),
        }
    }

    #[test]
    fn a_mode_that_is_probably_a_mistake_is_said() {
        assert_eq!(warned("501"), [ModeWarning::OthersOverGroup]);
        assert_eq!(
            warned("467"),
            [
                ModeWarning::OthersOverGroup,
                ModeWarning::GroupOverOwner,
                ModeWarning::WorldWritable
            ]
        );
        assert_eq!(warned("666"), [ModeWarning::WorldWritable]);
        assert_eq!(
            warned("4644"),
            [ModeWarning::SpecialWithoutExecute],
            "rwSr--r--"
        );
        assert_eq!(
            warned("2745"),
            [
                ModeWarning::OthersOverGroup,
                ModeWarning::SpecialWithoutExecute
            ]
        );
    }

    #[test]
    fn the_usual_modes_draw_no_warning() {
        for usual in [
            "755",
            "644",
            "600",
            "700",
            "750",
            "640",
            "4755",
            "2775",
            "1777",
            "rwxr-xr-x",
        ] {
            assert_eq!(warned(usual), [], "{usual}");
        }
    }

    #[test]
    fn octal_and_symbolic_are_one_mode() {
        let (octal, _) = mode("755");
        let (symbolic, _) = mode("rwxr-xr-x");

        assert_eq!(octal, symbolic);
        assert_eq!(octal.bits, 0o755);
        assert_eq!(octal.octal, "755");
        assert_eq!(octal.symbolic, "rwxr-xr-x");
        assert_eq!(octal.chmod_symbolic, "u=rwx,g=rx,o=rx");
        assert_eq!(
            octal.classes[2],
            ClassRights {
                class: Class::Others,
                read: true,
                write: false,
                execute: true,
                special: false,
            }
        );
        assert_eq!(mode("0644").0.symbolic, "rw-r--r--");
        assert_eq!(mode("0o600").0.octal, "600");
        assert_eq!(mode("0").0.chmod_symbolic, "u=,g=,o=");
    }

    #[test]
    fn every_special_bit_both_ways() {
        for (octal, symbolic, chmod) in [
            ("4755", "rwsr-xr-x", "u=rwxs,g=rx,o=rx"),
            ("4644", "rwSr--r--", "u=rws,g=r,o=r"),
            ("2755", "rwxr-sr-x", "u=rwx,g=rxs,o=rx"),
            ("2744", "rwxr-Sr--", "u=rwx,g=rs,o=r"),
            ("1777", "rwxrwxrwt", "u=rwx,g=rwx,o=rwxt"),
            ("1776", "rwxrwxrwT", "u=rwx,g=rwx,o=rwt"),
            ("7777", "rwsrwsrwt", "u=rwxs,g=rwxs,o=rwxt"),
        ] {
            let (from_octal, _) = mode(octal);
            let (from_symbolic, _) = mode(symbolic);
            assert_eq!(from_octal.symbolic, symbolic, "{octal}");
            assert_eq!(from_symbolic.octal, octal, "{symbolic}");
            assert_eq!(from_octal.chmod_symbolic, chmod, "{octal}");
        }
        assert!(mode("4755").0.classes[0].special);
    }

    #[test]
    fn the_file_type_of_ls_is_read_and_named() {
        for (text, file_type) in [
            ("-rw-r--r--", FileType::Regular),
            ("drwxr-xr-x", FileType::Directory),
            ("lrwxrwxrwx", FileType::SymbolicLink),
            ("crw-rw-rw-", FileType::CharacterDevice),
            ("brw-rw----", FileType::BlockDevice),
            ("prw-r--r--", FileType::NamedPipe),
            ("srwxrwxrwx", FileType::Socket),
            ("drwxr-xr-x.", FileType::Directory),
            ("-rw-r--r--@", FileType::Regular),
        ] {
            assert_eq!(mode(text).1, Some(file_type), "{text}");
        }
        assert_eq!(mode("rwxr-xr-x").1, None);
        assert_eq!(mode("drwxr-xr-x").0.octal, "755");
    }

    #[test]
    fn a_umask_gives_the_mode_of_a_new_file_and_a_new_directory() {
        let answer = describe(&PermissionsRequest {
            mode: String::new(),
            umask: "022".into(),
        });

        assert_eq!(answer.mode, ModeReading::Empty);
        let UmaskReading::Read { file, directory } = answer.umask else {
            panic!("read");
        };
        assert_eq!(
            (file.octal.as_str(), directory.octal.as_str()),
            ("644", "755")
        );
        let UmaskReading::Read { file, directory } = describe(&PermissionsRequest {
            mode: String::new(),
            umask: "0077".into(),
        })
        .umask
        else {
            panic!("read");
        };
        assert_eq!(
            (file.symbolic.as_str(), directory.symbolic.as_str()),
            ("rw-------", "rwx------")
        );
        assert_eq!(
            describe(&PermissionsRequest {
                mode: String::new(),
                umask: "028".into(),
            })
            .umask,
            UmaskReading::Refused {
                problem: ModeProblem::NotOctal,
                at: 3
            }
        );
    }

    #[test]
    fn every_refusal_says_where() {
        assert_eq!(refusal("758"), (ModeProblem::NotOctal, 3));
        assert_eq!(refusal("75x"), (ModeProblem::Unreadable, 3));
        assert_eq!(refusal("17777"), (ModeProblem::TooLarge, 1));
        assert_eq!(refusal("rwxr-xr-"), (ModeProblem::Length, 1));
        assert_eq!(refusal("rwxr-xr-xx"), (ModeProblem::Unexpected, 1));
        assert_eq!(refusal("rwxrwxr-q"), (ModeProblem::Unexpected, 9));
        assert_eq!(refusal("rwxr-tr-x"), (ModeProblem::Unexpected, 6));
        assert_eq!(refusal("rwxr-xr-s"), (ModeProblem::Unexpected, 9));
        assert_eq!(refusal("  drwxr-xrw!"), (ModeProblem::Unexpected, 12));
        assert_eq!(refusal("wrxr-xr-x"), (ModeProblem::Unexpected, 1));
    }
}
