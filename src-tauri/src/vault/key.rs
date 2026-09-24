//! Deriving the key from a passphrase, and sealing a value with it.
//!
//! AES-256-GCM hides a value's content, never its length: the file says how big each note
//! is. ⚠️ The nonce is 96 fresh random bits per seal and never the caller's — reusing one
//! under the same key breaks GCM outright.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use zeroize::Zeroizing;

use crate::error::StorageError;

/// 96 bits, what GCM is specified around.
const NONCE_BYTES: usize = 12;
/// What sealing adds to a value: the nonce in front, the GCM tag behind.
pub(crate) const SEALED_OVERHEAD: usize = NONCE_BYTES + 16;
/// 128 bits: the salt only has to be unique per library, never secret.
pub(crate) const SALT_BYTES: usize = 16;

/// What `derive` costs, and what an attacker guessing passphrases pays per guess.
///
/// ⚠️ Stored in the key file, not read from constants: raising the cost in a later version
/// must not lock existing libraries out. Only a deliberate re-key changes a file's cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cost {
    /// KiB of memory. The parameter that actually hurts a GPU.
    pub memory_kib: u32,
    pub passes: u32,
    pub lanes: u32,
}

impl Cost {
    /// For keys that live as long as a test or a benchmark.
    #[doc(hidden)]
    pub const FOR_TESTS: Self = Self {
        memory_kib: 64,
        passes: 1,
        lanes: 1,
    };
}

impl Default for Cost {
    /// Paid once per unlock. The 64 MiB is what bounds an attacker, three times OWASP's floor;
    /// the time — about 50 ms in a release build — never carried the defence.
    fn default() -> Self {
        Self {
            memory_kib: 64 * 1024,
            passes: 3,
            lanes: 1,
        }
    }
}

/// The key, wiped from memory when the vault is dropped.
pub struct Vault {
    key: Zeroizing<[u8; 32]>,
}

impl std::fmt::Debug for Vault {
    /// Says nothing: a key that reaches a log is a key that is gone.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Vault(…)")
    }
}

impl Vault {
    /// The key the library's values are actually sealed with — random, never derived.
    ///
    /// The phrase only wraps this key, which makes changing it a hundred bytes of rewriting
    /// rather than re-encrypting the corpus — and means a changed phrase answers a leaked
    /// phrase, never a leaked key.
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

    /// The only check there is: a wrong phrase is one whose key the tag refuses to open.
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

    /// Slow on purpose, and paid once per unlock, never per value.
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

    /// `nonce ++ ciphertext ++ tag`, raw: what a file holds. A column holds its base64.
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

    /// Whole, never streamed: GCM authenticates a message only once all of it has been seen.
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

    /// The same bytes, base64, for the TEXT columns.
    pub fn seal(&self, plaintext: &str) -> Result<String, StorageError> {
        Ok(BASE64.encode(self.seal_bytes(plaintext.as_bytes())?))
    }

    /// Fails on a value sealed with another key: the tag is what tells a wrong passphrase or a
    /// tampered file, and neither is guessed at.
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

    /// Otherwise the file would say which notes are identical.
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

    /// What tells a wrong passphrase from a right one.
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

    /// A flipped byte is refused rather than decrypted into plausible nonsense.
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

    #[test]
    fn a_fresh_salt_is_not_the_previous_one() {
        assert_ne!(fresh_salt().unwrap(), fresh_salt().unwrap());
    }

    #[test]
    fn a_vault_never_prints_its_key() {
        assert_eq!(format!("{:?}", vault()), "Vault(…)");
    }
}
