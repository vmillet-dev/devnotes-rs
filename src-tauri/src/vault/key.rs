//! Deriving the key from a passphrase, and sealing a value with it.
//!
//! ⚠️ AES-256-GCM hides the content of a value, never its **length**: the ciphertext is as
//! long as the plaintext. Someone holding the file learns how big each note is, and that a
//! title is empty. That is inherent to the construction and is not worth padding around.
//!
//! ⚠️ The nonce is 96 bits of randomness, fresh per seal. Reusing one under the same key
//! breaks GCM outright, so nothing here ever takes a nonce from the caller.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use zeroize::Zeroizing;

use crate::error::StorageError;

/// 96 bits, what GCM is specified around.
const NONCE_BYTES: usize = 12;
/// 128 bits: the salt only has to be unique per library, never secret.
pub(crate) const SALT_BYTES: usize = 16;

/// What `derive` costs, and what an attacker guessing passphrases pays per guess.
///
/// ⚠️ These travel **in the key file**, not as constants read at derivation: raising them
/// in a later version must not lock every existing library out. A file carries the
/// parameters it was written with, and only a deliberate re-key changes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cost {
    /// KiB of memory. The parameter that actually hurts a GPU.
    pub memory_kib: u32,
    pub passes: u32,
    pub lanes: u32,
}

impl Cost {
    /// A cost nobody would ship, for keys that live as long as a test or a benchmark and
    /// never guard a file anyone keeps.
    #[doc(hidden)]
    pub const FOR_TESTS: Self = Self {
        memory_kib: 64,
        passes: 1,
        lanes: 1,
    };
}

impl Default for Cost {
    /// Paid **once per launch**, and the only thing standing between a copied library and
    /// someone working through a wordlist.
    ///
    /// ⚠️ **52.6 ms per derivation in a release build** (`cargo bench -- unlock`), not the
    /// ~1.2 s this claimed for a while — that figure came from `cargo test`, where Argon2
    /// is unoptimised, and the two differ by twenty. What bounds an attacker here is the
    /// 64 MiB, three times OWASP's floor and the parameter a GPU cannot buy its way out of;
    /// the time never carried the defence, which is why the real number changes nothing.
    ///
    /// Raising it later locks nobody out: the cost travels in the key file, and a library
    /// reopens at whatever it was written with.
    fn default() -> Self {
        Self {
            memory_kib: 64 * 1024,
            passes: 3,
            lanes: 1,
        }
    }
}

/// The key, and nothing else. ⚠️ Held in a `Zeroizing` so it is wiped when the vault is
/// dropped rather than left in freed memory for whatever reads it next.
pub struct Vault {
    key: Zeroizing<[u8; 32]>,
}

impl std::fmt::Debug for Vault {
    /// ⚠️ Deliberately says nothing: a key that reaches a log is a key that is gone.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Vault(…)")
    }
}

impl Vault {
    /// The key the library's values are actually sealed with — random, never derived.
    ///
    /// ⚠️ This is what makes changing the passphrase a hundred bytes of rewriting instead
    /// of re-encrypting every note and every attachment: the phrase only ever protects
    /// this key, and a new phrase wraps the same one again. The cost is the other side of
    /// that coin — whoever gets hold of this key keeps access across a change, so a
    /// changed passphrase answers a leaked *phrase*, never a leaked key.
    pub fn random() -> Result<Self, StorageError> {
        let mut key = Zeroizing::new([0u8; 32]);
        getrandom::fill(key.as_mut())
            .map_err(|error| StorageError::Vault(format!("no randomness: {error}")))?;

        Ok(Self { key })
    }

    /// Seals this key under another one, for the key file to hold.
    pub fn wrapped_with(&self, wrapping: &Self) -> Result<Vec<u8>, StorageError> {
        wrapping.seal_bytes(self.key.as_ref())
    }

    /// ⚠️ The other direction, and the only check there is: a phrase that does not open
    /// the wrapped key is the wrong phrase, said by the authentication tag rather than by
    /// a known value sealed beside it.
    pub fn unwrapped_with(wrapping: &Self, wrapped: &[u8]) -> Result<Self, StorageError> {
        let opened = Zeroizing::new(wrapping.open_bytes(wrapped)?);
        let key: [u8; 32] = opened
            .as_slice()
            .try_into()
            .map_err(|_| StorageError::Vault("the wrapped key is the wrong size".to_string()))?;

        Ok(Self {
            key: Zeroizing::new(key),
        })
    }

    /// ⚠️ Slow on purpose — this is the whole defence against someone trying passphrases
    /// against a copied file. It is paid once, at unlock, never per value.
    pub fn derive(passphrase: &str, salt: &[u8], cost: Cost) -> Result<Self, StorageError> {
        let params = Params::new(cost.memory_kib, cost.passes, cost.lanes, Some(32))
            .map_err(|error| StorageError::Vault(format!("key parameters: {error}")))?;

        let mut key = Zeroizing::new([0u8; 32]);
        Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
            .hash_password_into(passphrase.as_bytes(), salt, key.as_mut())
            .map_err(|error| StorageError::Vault(format!("key derivation: {error}")))?;

        Ok(Self { key })
    }

    fn cipher(&self) -> Aes256Gcm {
        Aes256Gcm::new(&Key::<Aes256Gcm>::from(*self.key))
    }

    /// `nonce ++ ciphertext ++ tag`, raw. What a file holds — a column holds the base64 of
    /// this, because the columns are TEXT.
    pub fn seal_bytes(&self, plaintext: &[u8]) -> Result<Vec<u8>, StorageError> {
        let mut nonce = [0u8; NONCE_BYTES];
        getrandom::fill(&mut nonce)
            .map_err(|error| StorageError::Vault(format!("no randomness: {error}")))?;

        let sealed = self
            .cipher()
            .encrypt((&nonce).into(), plaintext)
            .map_err(|_| StorageError::Vault("could not seal a value".to_string()))?;

        let mut joined = Vec::with_capacity(NONCE_BYTES + sealed.len());
        joined.extend_from_slice(&nonce);
        joined.extend_from_slice(&sealed);

        Ok(joined)
    }

    /// ⚠️ Whole, never streamed: GCM only authenticates a message once all of it has been
    /// seen, and handing back bytes before the tag is checked would defeat the point.
    pub fn open_bytes(&self, sealed: &[u8]) -> Result<Vec<u8>, StorageError> {
        if sealed.len() <= NONCE_BYTES {
            return Err(StorageError::Vault(
                "too short to have been sealed".to_string(),
            ));
        }

        let (nonce, body) = sealed.split_at(NONCE_BYTES);
        let nonce: &[u8; NONCE_BYTES] = nonce.try_into().expect("a checked length");

        self.cipher()
            .decrypt(nonce.into(), body)
            .map_err(|_| StorageError::Vault("it would not open".to_string()))
    }

    /// The same bytes, base64. The columns are TEXT, so what goes in one has to survive
    /// being read back as a string.
    pub fn seal(&self, plaintext: &str) -> Result<String, StorageError> {
        Ok(BASE64.encode(self.seal_bytes(plaintext.as_bytes())?))
    }

    /// ⚠️ Fails on a value that was not sealed with this key, and that is the point: the
    /// tag is what tells a wrong passphrase from a tampered file. Neither is recoverable,
    /// so neither is guessed at.
    pub fn open(&self, sealed: &str) -> Result<String, StorageError> {
        let raw = BASE64
            .decode(sealed)
            .map_err(|_| StorageError::Vault("a value is not base64".to_string()))?;

        String::from_utf8(self.open_bytes(&raw)?).map_err(|_| {
            StorageError::Vault("a value opened to something that is not text".to_string())
        })
    }
}

/// A salt for a library that has none yet.
pub fn fresh_salt() -> Result<[u8; SALT_BYTES], StorageError> {
    let mut salt = [0u8; SALT_BYTES];
    getrandom::fill(&mut salt)
        .map_err(|error| StorageError::Vault(format!("no randomness: {error}")))?;

    Ok(salt)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⚠️ Cheap parameters, and only here: deriving at the real cost in every test would
    /// add minutes to `cargo test` to prove nothing the real parameters prove better.
    fn vault() -> Vault {
        Vault::derive("a passphrase", b"0123456789abcdef", Cost::FOR_TESTS).unwrap()
    }

    #[test]
    fn a_sealed_value_opens_back_to_itself() {
        let vault = vault();

        let sealed = vault.seal("psql -h prod.internal -U admin").unwrap();

        assert_eq!(
            vault.open(&sealed).unwrap(),
            "psql -h prod.internal -U admin"
        );
    }

    #[test]
    fn an_empty_value_survives_the_round_trip() {
        let vault = vault();

        let sealed = vault.seal("").unwrap();

        assert_eq!(vault.open(&sealed).unwrap(), "");
    }

    #[test]
    fn accents_and_emoji_come_back_whole() {
        let vault = vault();
        let text = "étape de déploiement ⚡ 日本語";

        assert_eq!(vault.open(&vault.seal(text).unwrap()).unwrap(), text);
    }

    /// The same text twice must not produce the same ciphertext, or the file would say
    /// which notes are identical.
    #[test]
    fn sealing_the_same_text_twice_gives_two_different_values() {
        let vault = vault();

        assert_ne!(vault.seal("same").unwrap(), vault.seal("same").unwrap());
    }

    #[test]
    fn the_sealed_form_carries_none_of_the_plaintext() {
        let vault = vault();

        let sealed = vault.seal("hunter2").unwrap();

        assert!(!sealed.contains("hunter2"));
    }

    /// ⚠️ What tells a wrong passphrase from a right one. Without it an unlock would
    /// succeed and the library would read as gibberish.
    #[test]
    fn a_value_will_not_open_under_another_passphrase() {
        let sealed = vault().seal("secret").unwrap();
        let other =
            Vault::derive("another passphrase", b"0123456789abcdef", Cost::FOR_TESTS).unwrap();

        assert!(other.open(&sealed).is_err());
    }

    #[test]
    fn the_same_passphrase_under_another_salt_is_another_key() {
        let sealed = vault().seal("secret").unwrap();
        let elsewhere =
            Vault::derive("a passphrase", b"fedcba9876543210", Cost::FOR_TESTS).unwrap();

        assert!(elsewhere.open(&sealed).is_err());
    }

    /// ⚠️ GCM authenticates: a flipped byte is refused rather than decrypted into
    /// plausible-looking nonsense.
    #[test]
    fn a_tampered_value_is_refused_rather_than_opened() {
        let vault = vault();
        let sealed = vault.seal("the original").unwrap();

        let mut raw = BASE64.decode(&sealed).unwrap();
        let last = raw.len() - 1;
        raw[last] ^= 0x01;

        assert!(vault.open(&BASE64.encode(raw)).is_err());
    }

    #[test]
    fn anything_that_is_not_a_sealed_value_is_refused() {
        let vault = vault();

        assert!(vault.open("not base64 at all !!").is_err());
        assert!(vault.open("").is_err());
        assert!(vault.open(&BASE64.encode([0u8; 4])).is_err());
    }

    /// Two salts drawn in a row must differ, or every library would share a key.
    #[test]
    fn a_fresh_salt_is_not_the_previous_one() {
        assert_ne!(fresh_salt().unwrap(), fresh_salt().unwrap());
    }

    /// ⚠️ A key that reaches a log is a key that is gone.
    #[test]
    fn a_vault_never_prints_its_key() {
        assert_eq!(format!("{:?}", vault()), "Vault(…)");
    }
}
