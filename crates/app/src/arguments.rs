//! The command line. Only tests and development pass anything.

use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Arguments {
    /// A directory holding the whole profile instead of the installed one.
    pub(crate) profile: Option<PathBuf>,
}

/// What is not understood is left alone: a shortcut or an autostart entry may carry more.
pub(crate) fn read(arguments: impl IntoIterator<Item = OsString>) -> Arguments {
    let mut read = Arguments::default();
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        if argument == "--profile" {
            read.profile = arguments.next().map(PathBuf::from);
        }
    }
    read
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(arguments: &[&str]) -> Arguments {
        read(arguments.iter().map(OsString::from))
    }

    #[test]
    fn a_profile_directory_is_read_after_its_flag() {
        assert_eq!(
            of(&["--profile", "/tmp/run-1"]).profile,
            Some(PathBuf::from("/tmp/run-1"))
        );
    }

    #[test]
    fn no_flag_is_the_installed_profile() {
        assert_eq!(of(&[]), Arguments::default());
        assert_eq!(of(&["--profile"]).profile, None);
    }

    #[test]
    fn an_unknown_argument_is_left_alone() {
        assert_eq!(
            of(&["--minimized", "--profile", "p"]).profile,
            Some(PathBuf::from("p"))
        );
    }
}
