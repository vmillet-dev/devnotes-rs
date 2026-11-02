//! Finding a request from the palette: by its name, its URL as typed, or where it is filed.
//! Ranked here, as the notes are searched here: the front draws the order it is given.

use chrono::{DateTime, Utc};
use serde::Serialize;
use specta::Type;

use super::model::{HttpMethod, RequestKind};
use crate::notes::view::fold;

/// Beside the notes, the tools and the spaces, more would not fit on screen.
pub const MAX_HITS: usize = 8;

/// A request as the palette shows it: its URL as typed, `{{baseUrl}}` and all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RequestHit {
    pub id: String,
    pub name: String,
    pub kind: RequestKind,
    pub method: HttpMethod,
    pub url: String,
    /// The collection, then each folder down to the request's.
    pub place: Vec<String>,
}

/// A request and when it was last edited, which breaks a tie and orders an empty search.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub hit: RequestHit,
    pub updated_at: DateTime<Utc>,
}

/// How well a request answers, lower first: its name before its URL before its place, and a
/// start before a middle. `None` when nothing of it holds what was typed.
fn score(hit: &RequestHit, needle: &str) -> Option<u8> {
    let name = fold(&hit.name);
    if name.starts_with(needle) {
        Some(0)
    } else if name
        .split(|character: char| !character.is_alphanumeric())
        .any(|word| word.starts_with(needle))
    {
        Some(1)
    } else if name.contains(needle) {
        Some(2)
    } else if fold(&hit.url).contains(needle) {
        Some(3)
    } else if hit.place.iter().any(|part| fold(part).contains(needle)) {
        Some(4)
    } else {
        None
    }
}

/// Nothing typed lists the requests edited last.
pub fn rank(candidates: Vec<Candidate>, search: &str) -> Vec<RequestHit> {
    let needle = fold(search.trim());
    let mut scored: Vec<(u8, DateTime<Utc>, RequestHit)> = candidates
        .into_iter()
        .filter_map(|candidate| {
            let score = if needle.is_empty() {
                Some(0)
            } else {
                score(&candidate.hit, &needle)
            };
            score.map(|score| (score, candidate.updated_at, candidate.hit))
        })
        .collect();
    scored.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(b.1.cmp(&a.1))
            .then_with(|| a.2.name.cmp(&b.2.name))
    });
    scored
        .into_iter()
        .take(MAX_HITS)
        .map(|(_, _, hit)| hit)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(name: &str, url: &str, place: &[&str], minutes: i64) -> Candidate {
        Candidate {
            hit: RequestHit {
                id: name.to_string(),
                name: name.to_string(),
                kind: RequestKind::Http,
                method: HttpMethod::Get,
                url: url.to_string(),
                place: place.iter().map(ToString::to_string).collect(),
            },
            updated_at: DateTime::UNIX_EPOCH + chrono::Duration::minutes(minutes),
        }
    }

    fn names(hits: &[RequestHit]) -> Vec<&str> {
        hits.iter().map(|hit| hit.name.as_str()).collect()
    }

    fn library() -> Vec<Candidate> {
        vec![
            candidate(
                "Lister les factures",
                "{{baseUrl}}/invoices",
                &["API Paiements", "Factures"],
                1,
            ),
            candidate(
                "Facture par id",
                "{{baseUrl}}/invoices/{{id}}",
                &["API Paiements", "Factures"],
                2,
            ),
            candidate(
                "Login",
                "{{baseUrl}}/auth/login",
                &["API Paiements", "Auth"],
                3,
            ),
            candidate(
                "Créer une facture",
                "{{baseUrl}}/invoices",
                &["API Paiements", "Factures"],
                4,
            ),
            candidate(
                "Santé",
                "https://status.exemple.fr/health",
                &["Supervision"],
                5,
            ),
        ]
    }

    #[test]
    fn a_name_starting_with_the_search_comes_before_a_word_of_one() {
        let hits = rank(library(), "fact");

        assert_eq!(
            names(&hits),
            ["Facture par id", "Créer une facture", "Lister les factures"]
        );
    }

    #[test]
    fn the_url_and_the_place_answer_after_the_name_accents_and_case_aside() {
        assert_eq!(names(&rank(library(), "AUTH/LOG")), ["Login"]);
        assert_eq!(names(&rank(library(), "supervision")), ["Santé"]);
        assert_eq!(names(&rank(library(), "sante")), ["Santé"]);
    }

    #[test]
    fn nothing_typed_lists_the_requests_edited_last() {
        let hits = rank(library(), "  ");

        assert_eq!(names(&hits)[..2], ["Santé", "Créer une facture"]);
    }

    #[test]
    fn a_search_answering_nothing_answers_nothing() {
        assert!(rank(library(), "graphql").is_empty());
    }

    #[test]
    fn no_more_than_the_palette_shows() {
        let many = (0..20)
            .map(|index| candidate(&format!("GET {index}"), "/", &["C"], index))
            .collect();

        assert_eq!(rank(many, "get").len(), MAX_HITS);
    }
}
