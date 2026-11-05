//! The HTTP collections, their requests, and what is sent.

use engine::http::{self, model::HttpTree};

use crate::backend::{Backend, Pending};

pub fn tree(backend: &Backend) -> Pending<HttpTree> {
    backend.db(http::http_tree)
}
