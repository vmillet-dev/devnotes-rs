//! The gate: what the library is, creating it, unlocking it. Deriving the key is slow on
//! purpose, so it is never asked of the UI thread.

use std::sync::Arc;

use engine::vault::{self, VaultState};
use engine::{db, index};
use zeroize::Zeroizing;

use crate::backend::{Backend, Pending};

pub fn state(backend: &Backend) -> Pending<VaultState> {
    backend.db(vault::vault_state)
}

/// The first launch. The index fills behind the gate, a slice a job of the queue.
pub fn create(backend: &Backend, passphrase: Zeroizing<String>) -> Pending<()> {
    let filling = backend.background();
    backend.spawn(move |engine| async move {
        db::run(Arc::clone(&engine), move |engine| {
            vault::create_vault(engine, passphrase)?;
            index::announce(engine)
        })
        .await?;
        filling.run(index::fill(engine));
        Ok(())
    })
}

pub fn unlock(backend: &Backend, passphrase: Zeroizing<String>) -> Pending<()> {
    let filling = backend.background();
    backend.spawn(move |engine| async move {
        db::run(Arc::clone(&engine), move |engine| {
            vault::unlock_vault(engine, passphrase)?;
            index::announce(engine)
        })
        .await?;
        filling.run(index::fill(engine));
        Ok(())
    })
}

/// A new phrase over the library open: the key is rewrapped, the notes are not touched.
pub fn change_passphrase(
    backend: &Backend,
    current: Zeroizing<String>,
    next: Zeroizing<String>,
) -> Pending<vault::PassphraseChange> {
    backend.db(move |engine| vault::change_passphrase(engine, current, next))
}

/// A library SQLite refuses to read, moved aside: the folder it went to.
pub fn set_aside(backend: &Backend) -> Pending<String> {
    backend.db(engine::recovery::set_aside_damaged_library)
}

/// A library whose phrase is forgotten, archived sealed: the folder it went to.
pub fn archive(backend: &Backend) -> Pending<String> {
    backend.db(engine::recovery::archive_locked_library)
}

#[cfg(test)]
mod tests {
    use engine::error::ErrorCode;

    use super::*;
    use crate::testing;

    fn settled<T>(
        backend: &Backend,
        mut pending: Pending<T>,
    ) -> Result<T, engine::error::AppError> {
        backend.settle();
        pending.poll().expect("answered once settled")
    }

    #[test]
    fn a_created_library_is_open() {
        let (backend, _profile) = testing::unlocked();

        assert_eq!(
            settled(&backend, state(&backend)).unwrap(),
            VaultState::Unlocked
        );
    }

    #[test]
    fn a_library_reopens_with_its_phrase_and_only_with_it() {
        let profile = testing::locked();
        let backend = testing::backend_on(profile.path());
        assert_eq!(
            settled(&backend, state(&backend)).unwrap(),
            VaultState::Locked
        );

        let wrong = Zeroizing::new("not the phrase at all".to_string());
        let refused = settled(&backend, unlock(&backend, wrong)).unwrap_err();
        assert_eq!(refused.code, ErrorCode::WrongPassphrase);

        let right = Zeroizing::new(testing::PASSPHRASE.to_string());
        settled(&backend, unlock(&backend, right)).unwrap();
        assert_eq!(
            settled(&backend, state(&backend)).unwrap(),
            VaultState::Unlocked
        );
    }
}
