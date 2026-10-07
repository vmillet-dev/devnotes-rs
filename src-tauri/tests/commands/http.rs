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

    let broken = session
        .call(|_| {
            describe_http_body(RequestBody::Json {
                text: "{\"a\":".to_string(),
            })
        })
        .unwrap();
    assert_eq!(broken.content_type.as_deref(), Some("application/json"));
    assert!(broken.problem.is_some());
    let none = session
        .call(|_| describe_http_body(RequestBody::None))
        .unwrap();
    assert_eq!((none.content_type, none.problem), (None, None));
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

#[test]
fn a_request_turned_into_graphql_keeps_its_kind_and_its_query() {
    use devnotes_lib::http::graphql::GraphqlDocument;

    let session = Session::open();
    let api = session
        .call(|app| create_http_collection("API".to_string(), app))
        .unwrap();
    let created = session
        .call(|app| create_http_request(draft(&api.id, None, "Produits"), app))
        .unwrap();
    let patch = HttpRequestPatch {
        kind: Some(RequestKind::Graphql),
        document: Some(RequestDocument {
            graphql: GraphqlDocument {
                query: "{ products { id } }".to_string(),
                ..GraphqlDocument::default()
            },
            ..RequestDocument::default()
        }),
        ..HttpRequestPatch::default()
    };

    let saved = session
        .call(|app| save_http_request(created.id.clone(), patch, app))
        .unwrap();

    assert_eq!(saved.kind, RequestKind::Graphql);
    let read = session
        .call(|app| get_http_request(created.id.clone(), app))
        .unwrap();
    assert_eq!(read.document.graphql.query, "{ products { id } }");
}
