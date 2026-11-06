//! A link written in a note, for the browser. The web only: a note is anyone's text, and a
//! `file:` link is a program to run.

use std::path::Path;

use crate::desktop::Opener;

pub fn is_web(url: &str) -> bool {
    let scheme = url.split_once("://").map_or("", |(scheme, _)| scheme);
    scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")
}

/// `false` when it was refused, or the desktop would not open it.
pub fn open(opener: &dyn Opener, url: &str) -> bool {
    is_web(url) && opener.open(Path::new(url)).is_ok()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io;

    use super::*;

    #[derive(Default)]
    struct Opened(RefCell<Vec<String>>);

    impl Opener for Opened {
        fn open(&self, path: &Path) -> io::Result<()> {
            self.0.borrow_mut().push(path.display().to_string());
            Ok(())
        }
    }

    #[test]
    fn the_web_opens_whatever_the_case_of_its_scheme() {
        let opened = Opened::default();

        assert!(open(&opened, "https://exemple.fr/doc"));
        assert!(open(&opened, "HTTP://exemple.fr"));
        assert_eq!(opened.0.borrow().len(), 2);
    }

    #[test]
    fn anything_else_never_reaches_the_desktop() {
        let opened = Opened::default();

        for url in [
            "file:///C:/Windows/System32/calc.exe",
            "javascript:alert(1)",
            "ftp://exemple.fr",
            "C:\\Windows\\notepad.exe",
            "https:exemple.fr",
            "",
        ] {
            assert!(!open(&opened, url), "{url}");
        }
        assert!(opened.0.borrow().is_empty());
    }
}
