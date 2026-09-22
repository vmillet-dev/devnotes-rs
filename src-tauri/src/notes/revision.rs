//! The body as it was before an edit.
//!
//! ⚠️ The trash protects a deletion; nothing protected an edit. You adjust a command that
//! worked, it stops working, and the version that worked is gone. The point is not the
//! restoring — it is the **ease**: a text you know is recoverable is a text you edit
//! freely, and touching a snippet that works stops costing nerve.

use chrono::{DateTime, Utc};
use serde::Serialize;
use specta::Type;

/// How many bodies are kept per note.
///
/// ⚠️ A **count**, not a time window. A body runs to tens of kilobytes, so a cap is the
/// only bound that is predictable for storage — the trash's thirty days is a precedent
/// for the shape of the retention, not for its unit.
pub const KEEP: usize = 20;

/// One kept body, as the panel lists it.
///
/// ⚠️ Metadata only. The bodies are what makes this table big, and a list that carried
/// twenty of them would send the whole history across to draw twenty dates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Revision {
    pub id: String,
    pub taken_at: DateTime<Utc>,
    /// So a row can say how much a version held without carrying it.
    pub characters: u32,
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn describe(id: String, taken_at: DateTime<Utc>, content: &str) -> Revision {
    Revision {
        id,
        taken_at,
        // Saturated rather than wrapped: a body of four billion characters is not a body,
        // and the count is a label on a row.
        characters: content.chars().count().min(u32::MAX as usize) as u32,
    }
}
