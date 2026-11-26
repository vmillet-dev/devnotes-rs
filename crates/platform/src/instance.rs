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

    /// Any process of the machine can open the channel: an action is heard by the `e2e` flavour
    /// alone, or anyone could open the palette, capture the clipboard or make a note.
    fn read(line: &str) -> Option<Self> {
        match line.trim_end().split_once(' ') {
            None if line.trim_end() == "reveal" => Some(Self::Reveal),
            Some(("action", name)) if cfg!(feature = "e2e") && !name.is_empty() => {
                Some(Self::Action(name.to_string()))
            }
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

pub struct Instance {
    listener: Listener,
    #[cfg(target_os = "macos")]
    _lock: std::fs::File,
}

/// Takes the profile's name, or finds it taken. Two profiles never share one.
pub fn claim(profile: &Path) -> io::Result<Claim> {
    let name = socket_name(profile)?.to_ns_name::<GenericNamespaced>()?;
    // ⚠️ macOS names a socket by a file, which a crash leaves behind: there a lock, which goes with
    // the process, says who holds the profile, and a name left behind is taken over.
    #[cfg(target_os = "macos")]
    let Some(lock) = lock(profile)? else {
        return Ok(Claim::Taken);
    };
    let options = ListenerOptions::new()
        .name(name)
        .try_overwrite(cfg!(target_os = "macos"));
    match options.create_sync() {
        Ok(listener) => Ok(Claim::First(Instance {
            listener,
            #[cfg(target_os = "macos")]
            _lock: lock,
        })),
        // ⚠️ Windows answers a named pipe already created with « access denied », not
        // « address in use ».
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

/// `None` while another process holds it.
#[cfg(target_os = "macos")]
fn lock(profile: &Path) -> io::Result<Option<std::fs::File>> {
    let file = std::fs::File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(profile.join("instance.lock"))?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(std::fs::TryLockError::Error(error)) => Err(error),
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
                // Whole: naming the listener alone would leave the lock behind, and free it.
                let instance = self;
                for connection in instance.listener.incoming() {
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
        if cfg!(feature = "e2e") {
            assert_eq!(
                hearing.recv_timeout(wait).unwrap(),
                Message::Action("palette".to_string())
            );
        }
    }

    /// Any process of the machine can open the channel: outside the `e2e` flavour, it can
    /// show the window and nothing more.
    #[cfg(not(feature = "e2e"))]
    #[test]
    fn outside_the_e2e_flavour_a_connection_asks_for_no_action() {
        let profile = tempfile::tempdir().unwrap();
        let (heard, hearing) = mpsc::channel();
        first(profile.path())
            .serve(move |message| heard.send(message).unwrap())
            .unwrap();

        send(profile.path(), &Message::Action("palette".to_string())).unwrap();
        send(profile.path(), &Message::Reveal).unwrap();

        let wait = Duration::from_secs(10);
        assert_eq!(hearing.recv_timeout(wait).unwrap(), Message::Reveal);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_name_left_by_a_crash_is_taken_over() {
        let profile = tempfile::tempdir().unwrap();
        let name = socket_name(profile.path())
            .unwrap()
            .to_ns_name::<GenericNamespaced>()
            .unwrap();
        let crashed = ListenerOptions::new().name(name).reclaim_name(false);
        drop(crashed.create_sync().unwrap());

        drop(first(profile.path()));
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
        let action = cfg!(feature = "e2e").then(|| Message::Action("new-note".to_string()));
        assert_eq!(Message::read("action new-note\n"), action);
        assert_eq!(Message::read("action \n"), None);
        assert_eq!(Message::read("quit\n"), None);
    }
}
