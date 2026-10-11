use devnotes_lib::error::ErrorCode;
use devnotes_lib::http::model::{
    HttpItem, HttpItemKind, HttpMethod, HttpNode, HttpPlace, HttpRequestDraft, HttpRequestPatch,
    RequestDocument, RequestKind,
};
use devnotes_lib::http::{
    count_http_contents, create_http_collection, create_http_folder, create_http_request,
    delete_http_item, duplicate_http_item, get_http_request, http_tree, move_http_item,
    rename_http_item, reorder_http_collection, save_http_request,
};

use super::{Session, code};

fn item(kind: HttpItemKind, id: &str) -> HttpItem {
    HttpItem {
        kind,
        id: id.to_string(),
    }
}

fn draft(collection_id: &str, folder_id: Option<&str>, name: &str) -> HttpRequestDraft {
    HttpRequestDraft {
        collection_id: collection_id.to_string(),
        folder_id: folder_id.map(str::to_string),
        name: name.to_string(),
        kind: RequestKind::Http,
        method: HttpMethod::Get,
        document: RequestDocument {
            url: "https://api.exemple.fr".to_string(),
            ..RequestDocument::default()
        },
    }
}

fn names(session: &Session) -> Vec<String> {
    fn walk(nodes: &[HttpNode], into: &mut Vec<String>) {
        for node in nodes {
            match node {
                HttpNode::Folder { folder, children } => {
                    into.push(folder.name.clone());
                    walk(children, into);
                }
                HttpNode::Request { request } => into.push(request.name.clone()),
            }
        }
    }
    let mut seen = Vec::new();
    for node in session.call(http_tree).unwrap().collections {
        seen.push(node.collection.name);
        walk(&node.children, &mut seen);
    }
    seen
}

#[test]
fn a_collection_is_built_edited_moved_copied_counted_and_deleted() {
    let session = Session::open();
    let api = session
        .call(|app| create_http_collection("  API  ".to_string(), app))
        .unwrap();
    let other = session
        .call(|app| create_http_collection("Autre".to_string(), app))
        .unwrap();
    let auth = session
        .call(|app| create_http_folder(api.id.clone(), None, "Auth".to_string(), app))
        .unwrap();
    let login = session
        .call(|app| create_http_request(draft(&api.id, Some(&auth.id), "Login"), app))
        .unwrap();
    assert_eq!(names(&session), vec!["API", "Auth", "Login", "Autre"]);

    let patch = HttpRequestPatch {
        name: Some(" Se connecter ".to_string()),
        method: Some(HttpMethod::Post),
        ..HttpRequestPatch::default()
    };
    let saved = session
        .call(|app| save_http_request(login.id.clone(), patch, app))
        .unwrap();
    assert_eq!(
        (saved.name.as_str(), saved.method),
        ("Se connecter", HttpMethod::Post)
    );
    let read = session
        .call(|app| get_http_request(login.id.clone(), app))
        .unwrap();
    assert_eq!(read.document.url, "https://api.exemple.fr");

    session
        .call(|app| {
            rename_http_item(
                item(HttpItemKind::Folder, &auth.id),
                "Jetons".to_string(),
                app,
            )
        })
        .unwrap();
    let copy = session
        .call(|app| {
            duplicate_http_item(
                item(HttpItemKind::Request, &login.id),
                "Se connecter (copie)".to_string(),
                app,
            )
        })
        .unwrap();
    session
        .call(|app| {
            move_http_item(
                copy.clone(),
                HttpPlace {
                    collection_id: other.id.clone(),
                    folder_id: None,
                    index: 0,
                },
                app,
            )
        })
        .unwrap();
    session
        .call(|app| reorder_http_collection(other.id.clone(), 0, app))
        .unwrap();
    assert_eq!(
        names(&session),
        vec![
            "Autre",
            "Se connecter (copie)",
            "API",
            "Jetons",
            "Se connecter"
        ]
    );

    let contents = session
        .call(|app| count_http_contents(item(HttpItemKind::Collection, &api.id), app))
        .unwrap();
    assert_eq!((contents.folders, contents.requests), (1, 1));
    session
        .call(|app| delete_http_item(item(HttpItemKind::Collection, &api.id), app))
        .unwrap();
    assert_eq!(names(&session), vec!["Autre", "Se connecter (copie)"]);
}

#[test]
fn a_blank_name_a_missing_item_and_a_locked_library_are_refused_by_code() {
    let session = Session::open();
    assert_eq!(
        code(session.call(|app| create_http_collection("   ".to_string(), app))),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        code(session.call(|app| get_http_request("absent".to_string(), app))),
        ErrorCode::HttpItemNotFound
    );
    assert_eq!(
        code(session.call(|app| {
            rename_http_item(item(HttpItemKind::Request, "absent"), "x".to_string(), app)
        })),
        ErrorCode::HttpItemNotFound
    );
    assert_eq!(
        code(session.call(|app| {
            move_http_item(
                item(HttpItemKind::Collection, "c"),
                HttpPlace {
                    collection_id: "absent".to_string(),
                    folder_id: None,
                    index: 0,
                },
                app,
            )
        })),
        ErrorCode::HttpItemNotFound
    );

    assert_eq!(code(Session::locked().call(http_tree)), ErrorCode::Locked);
}

#[test]
fn a_folder_hands_its_settings_down_and_a_body_says_what_it_implies() {
    use devnotes_lib::http::model::{ContainerSettings, RequestAuth, RequestBody};
    use devnotes_lib::http::{
        describe_http_body, http_settings, inherited_http_settings, save_http_settings,
    };

    let session = Session::open();
    let api = session
        .call(|app| create_http_collection("API".to_string(), app))
        .unwrap();
    let factures = session
        .call(|app| create_http_folder(api.id.clone(), None, "Factures".to_string(), app))
        .unwrap();
    let bearer = RequestAuth::Bearer {
        token: "{{accessToken}}".to_string(),
    };
    let settings = ContainerSettings {
        auth: bearer.clone(),
        headers: Vec::new(),
        ..ContainerSettings::default()
    };
    session
        .call(|app| {
            save_http_settings(
                item(HttpItemKind::Folder, &factures.id),
                settings.clone(),
                app,
            )
        })
        .unwrap();

    let read = session
        .call(|app| http_settings(item(HttpItemKind::Folder, &factures.id), app))
        .unwrap();
    assert_eq!(read, settings);
    let inherited = session
        .call(|app| inherited_http_settings(api.id.clone(), Some(factures.id.clone()), app))
        .unwrap();
    assert_eq!(inherited.auth, bearer);
    assert_eq!(
        code(session.call(|app| http_settings(item(HttpItemKind::Request, "r"), app))),
        ErrorCode::InvalidInput
    );

    let json = || RequestBody::Json {
        text: "{\"a\":".to_string(),
    };
    let broken = session
        .call(|_| describe_http_body(json(), HttpMethod::Post))
        .unwrap();
    assert_eq!(broken.content_type.as_deref(), Some("application/json"));
    assert!(broken.problem.is_some());
    assert!(!broken.unusual);
    // The same body once the method is a `GET`: sent all the same, and said.
    for (method, unusual) in [
        (HttpMethod::Get, true),
        (HttpMethod::Head, true),
        (HttpMethod::Delete, false),
        (HttpMethod::Options, false),
    ] {
        let said = session
            .call(|_| describe_http_body(json(), method))
            .unwrap();
        assert_eq!(said.unusual, unusual, "{method:?}");
    }
    let none = session
        .call(|_| describe_http_body(RequestBody::None, HttpMethod::Get))
        .unwrap();
    assert_eq!(
        (none.content_type, none.problem, none.unusual),
        (None, None, false)
    );
}

/// Key `X-Recette: 1` in a folder's settings was stored as typed, and every request under the
/// folder then failed « this URL cannot be sent ».
#[test]
fn a_header_that_cannot_be_sent_is_refused_in_the_settings_marked_and_read_from_its_line() {
    use devnotes_lib::http::model::{ContainerSettings, HeaderFault, KeyValue};
    use devnotes_lib::http::{
        check_http_headers, http_settings, read_http_headers, save_http_settings,
    };

    let session = Session::open();
    let api = session
        .call(|app| create_http_collection("API".to_string(), app))
        .unwrap();
    let recette = session
        .call(|app| create_http_folder(api.id.clone(), None, "Recette".to_string(), app))
        .unwrap();
    let row = |key: &str, value: &str, enabled: bool| KeyValue {
        enabled,
        key: key.to_string(),
        value: value.to_string(),
        ..KeyValue::default()
    };
    let save = |headers: Vec<KeyValue>| {
        let settings = ContainerSettings {
            headers,
            ..ContainerSettings::default()
        };
        session
            .call(|app| save_http_settings(item(HttpItemKind::Folder, &recette.id), settings, app))
    };
    let kept = || {
        session
            .call(|app| http_settings(item(HttpItemKind::Folder, &recette.id), app))
            .unwrap()
            .headers
    };

    let refused = save(vec![row("X-Recette: 1", "rsgfdregr", true)]).unwrap_err();
    assert_eq!(refused.code, ErrorCode::HttpHeaderName);
    assert_eq!(
        (
            refused.params.get("header").map(String::as_str),
            refused.params.get("source").map(String::as_str)
        ),
        (Some("X-Recette: 1"), Some("request"))
    );
    assert!(kept().is_empty());

    let refused = save(vec![row("X-Trace", "a\nb", true)]).unwrap_err();
    assert_eq!(refused.code, ErrorCode::HttpHeaderValue);
    assert_eq!(
        refused.params.get("header").map(String::as_str),
        Some("X-Trace")
    );

    // Switched off, it is not sent: kept as typed, beside the one that is.
    save(vec![
        row("X-Recette: 1", "rsgfdregr", false),
        row("X-Recette", "1", true),
        row("X-{{tenant}}", "{{token}}", true),
    ])
    .unwrap();
    assert_eq!(kept().len(), 3);

    let faults = session
        .call(|_| {
            check_http_headers(vec![
                row("Accept", "*/*", true),
                row("X Recette", "1", true),
                row("X Recette", "1", false),
            ])
        })
        .unwrap();
    assert_eq!(faults, vec![None, Some(HeaderFault::Name), None]);

    let read = session
        .call(|_| {
            read_http_headers(":authority: api.exemple.fr\nX-Recette: 1\naccept: */*".to_string())
        })
        .unwrap();
    assert_eq!(
        read,
        vec![row("X-Recette", "1", true), row("accept", "*/*", true)]
    );
}

#[test]
fn the_query_and_its_table_are_kept_in_step_through_the_command() {
    use devnotes_lib::http::query::QuerySide;
    use devnotes_lib::http::sync_http_query;

    let session = Session::open();
    let synced = session
        .call(|_| sync_http_query("/x?a=1".to_string(), Vec::new(), QuerySide::Url))
        .unwrap();

    assert_eq!(synced.params[0].key, "a");
}

/// GraphQL and WebSocket were the last two entries of the method menu: a kind filed among
/// verbs. It is chosen when the request is created, and nothing changes it afterwards.
#[test]
fn a_request_is_created_of_its_kind_and_keeps_it_through_an_update_and_a_copy() {
    use devnotes_lib::http::graphql::GraphqlDocument;

    let session = Session::open();
    let api = session
        .call(|app| create_http_collection("API".to_string(), app))
        .unwrap();
    let asked = HttpRequestDraft {
        kind: RequestKind::Graphql,
        ..draft(&api.id, None, "Produits")
    };
    let created = session.call(|app| create_http_request(asked, app)).unwrap();
    assert_eq!(created.kind, RequestKind::Graphql);

    // An update has no kind to carry: one written on the wire all the same is not read.
    let mut patch: HttpRequestPatch =
        serde_json::from_str(r#"{"kind":"websocket","method":"POST"}"#).unwrap();
    patch.document = Some(RequestDocument {
        graphql: GraphqlDocument {
            query: "{ products { id } }".to_string(),
            ..GraphqlDocument::default()
        },
        ..RequestDocument::default()
    });
    let saved = session
        .call(|app| save_http_request(created.id.clone(), patch, app))
        .unwrap();
    assert_eq!(
        (saved.kind, saved.method),
        (RequestKind::Graphql, HttpMethod::Post)
    );
    let read = session
        .call(|app| get_http_request(created.id.clone(), app))
        .unwrap();
    assert_eq!(read.kind, RequestKind::Graphql);
    assert_eq!(read.document.graphql.query, "{ products { id } }");

    let copy = session
        .call(|app| {
            duplicate_http_item(
                item(HttpItemKind::Request, &created.id),
                "Produits (copie)".to_string(),
                app,
            )
        })
        .unwrap();
    let copied = session
        .call(|app| get_http_request(copy.id.clone(), app))
        .unwrap();
    assert_eq!(copied.kind, RequestKind::Graphql);
    assert_eq!(copied.document.graphql.query, "{ products { id } }");
}
