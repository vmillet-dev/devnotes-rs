//! One application per profile (D16). The in-memory index assumes no other process writes the
//! library: binding a local socket named after the profile makes the instance, and a second
//! process that cannot bind it hands its message to the first, then exits.

use std::fmt::Write as _;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;

use interprocess::local_socket::{
    GenericNamespaced, Listener, ListenerOptions, Stream, prelude::*,
};
use sha2::{Digest, Sha256};

/// What a second launch asks of the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Reveal,
    /// A global action by its name: only the `e2e` flavour sends one.
    Action(String),
}

impl Message {
    fn line(&self) -> String {
        match self {
            Self::Reveal => "reveal\n".to_string(),
            Self::Action(name) => format!("action {name}\n"),
        }
    }

    fn read(line: &str) -> Option<Self> {
        match line.trim_end().split_once(' ') {
            None if line.trim_end() == "reveal" => Some(Self::Reveal),
            Some(("action", name)) if !name.is_empty() => Some(Self::Action(name.to_string())),
            _ => None,
        }
    }
}

/// A message is a few words: anything longer is not one of ours.
const LONGEST_LINE: u64 = 256;

pub enum Claim {
    /// This process is the profile's instance; it hears the next launches through this.
    First(Instance),
    /// Another process holds the profile.
    Taken,
}

pub struct Instance(Listener);

/// Takes the profile's name, or finds it taken. Two profiles never share one.
pub fn claim(profile: &Path) -> io::Result<Claim> {
    let name = socket_name(profile)?.to_ns_name::<GenericNamespaced>()?;
    match ListenerOptions::new().name(name).create_sync() {
        Ok(listener) => Ok(Claim::First(Instance(listener))),
        // ⚠️ Windows answers a named pipe already created with « access denied », not
        // « address in use » (ADR S1).
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::AddrInUse | io::ErrorKind::PermissionDenied
            ) =>
        {
            Ok(Claim::Taken)
        }
        Err(error) => Err(error),
    }
}

/// What a second launch does: hand `message` to the profile's instance.
pub fn send(profile: &Path, message: &Message) -> io::Result<()> {
    let name = socket_name(profile)?.to_ns_name::<GenericNamespaced>()?;
    let mut stream = Stream::connect(name)?;
    stream.write_all(message.line().as_bytes())?;
    stream.flush()
}

impl Instance {
    /// Hears the next launches on a thread of its own, for as long as the process lives.
    pub fn serve(self, mut deliver: impl FnMut(Message) + Send + 'static) -> io::Result<()> {
        std::thread::Builder::new()
            .name("single-instance".to_string())
            .spawn(move || {
                for connection in self.0.incoming() {
                    match connection.map(read_message) {
                        Ok(Some(message)) => deliver(message),
                        Ok(None) => log::warn!("A second launch sent nothing readable"),
                        Err(error) => log::warn!("A second launch could not connect: {error}"),
                    }
                }
            })
            .map(drop)
    }
}

fn read_message(stream: Stream) -> Option<Message> {
    let mut line = String::new();
    BufReader::new(stream.take(LONGEST_LINE))
        .read_line(&mut line)
        .ok()?;
    Message::read(&line)
}

/// A digest of the canonical path: the same profile reached by two spellings is one name, and
/// a name says nothing of where the profile lives.
fn socket_name(profile: &Path) -> io::Result<String> {
    std::fs::create_dir_all(profile)?;
    let canonical = profile.canonicalize()?;
    let digest = Sha256::digest(canonical.as_os_str().as_encoded_bytes());
    let hex: String = digest[..12].iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    });
    Ok(format!("devnotes-{hex}"))
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;

    fn first(profile: &Path) -> Instance {
        match claim(profile).unwrap() {
            Claim::First(instance) => instance,
            Claim::Taken => panic!("the profile was already taken"),
        }
    }

    #[test]
    fn a_second_launch_on_the_same_profile_reaches_the_first() {
        let profile = tempfile::tempdir().unwrap();
        let (heard, hearing) = mpsc::channel();
        first(profile.path())
            .serve(move |message| heard.send(message).unwrap())
            .unwrap();

        assert!(matches!(claim(profile.path()).unwrap(), Claim::Taken));
        send(profile.path(), &Message::Reveal).unwrap();
        send(profile.path(), &Message::Action("palette".to_string())).unwrap();

        let wait = Duration::from_secs(10);
        assert_eq!(hearing.recv_timeout(wait).unwrap(), Message::Reveal);
        assert_eq!(
            hearing.recv_timeout(wait).unwrap(),
            Message::Action("palette".to_string())
        );
    }

    #[test]
    fn two_profiles_never_collide() {
        let one = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();

        let _held = first(one.path());

        assert!(matches!(claim(other.path()).unwrap(), Claim::First(_)));
    }

    #[test]
    fn two_spellings_of_one_profile_are_one_name() {
        let profile = tempfile::tempdir().unwrap();
        let roundabout = profile.path().join("nested").join("..");

        assert_eq!(
            socket_name(profile.path()).unwrap(),
            socket_name(&roundabout).unwrap()
        );
    }

    #[test]
    fn only_our_two_messages_are_read() {
        assert_eq!(Message::read("reveal\n"), Some(Message::Reveal));
        assert_eq!(
            Message::read("action new-note\n"),
            Some(Message::Action("new-note".to_string()))
        );
        assert_eq!(Message::read("action \n"), None);
        assert_eq!(Message::read("quit\n"), None);
    }
}
