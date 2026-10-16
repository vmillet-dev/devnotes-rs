//! The HTTP collections as data: a tree of collections, folders and requests, sealed, ordered
//! under each parent, moved, copied and deleted with what they hold.

use devnotes_lib::db::{Library, open_in_memory};
use devnotes_lib::error::StorageError;
use devnotes_lib::http::model::{
    HttpItem, HttpItemKind, HttpMethod, HttpNode, HttpPlace, HttpRequestDraft, HttpRequestPatch,
    HttpTree, RequestDocument, RequestKind,
};
use devnotes_lib::http::{search, store};

mod common;
use common::{t0, t1};

fn collection(connection: &mut Library, name: &str) -> String {
    store::create_collection(connection, name, t0()).unwrap().id
}

fn folder(
    connection: &mut Library,
    collection_id: &str,
    parent: Option<&str>,
    name: &str,
) -> String {
    store::create_folder(connection, collection_id, parent, name, t0())
        .unwrap()
        .id
}

fn request(
    connection: &mut Library,
    collection_id: &str,
    folder_id: Option<&str>,
    name: &str,
) -> String {
    let draft = HttpRequestDraft {
        collection_id: collection_id.to_string(),
        folder_id: folder_id.map(str::to_string),
        name: name.to_string(),
        kind: RequestKind::Http,
        method: HttpMethod::Get,
        document: RequestDocument {
            url: format!("https://api.exemple.fr/{name}"),
            ..RequestDocument::default()
        },
    };
    store::create_request(connection, &draft, name, t0())
        .unwrap()
        .id
}

fn item(kind: HttpItemKind, id: &str) -> HttpItem {
    HttpItem {
        kind,
        id: id.to_string(),
    }
}

fn place(collection_id: &str, folder_id: Option<&str>, index: u32) -> HttpPlace {
    HttpPlace {
        collection_id: collection_id.to_string(),
        folder_id: folder_id.map(str::to_string),
        index,
    }
}

/// The tree as names, a folder's children in brackets: `API[Auth[Login] Factures]`.
fn outline(tree: &HttpTree) -> Vec<String> {
    fn nodes(children: &[HttpNode]) -> String {
        children
            .iter()
            .map(|node| match node {
                HttpNode::Folder { folder, children } => {
                    format!("{}[{}]", folder.name, nodes(children))
                }
                HttpNode::Request { request } => request.name.clone(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
    tree.collections
        .iter()
        .map(|node| format!("{}[{}]", node.collection.name, nodes(&node.children)))
        .collect()
}

fn read(connection: &mut Library) -> Vec<String> {
    outline(&store::tree(connection).unwrap())
}

#[test]
fn a_collection_reads_back_as_a_tree_in_the_order_things_were_made() {
    let mut connection = open_in_memory().unwrap();
    let api = collection(&mut connection, "API Paiements");
    let auth = folder(&mut connection, &api, None, "Auth");
    request(&mut connection, &api, Some(&auth), "Login");
    let factures = folder(&mut connection, &api, None, "Factures");
    request(
        &mut connection,
        &api,
        Some(&factures),
        "Lister les factures",
    );
    request(&mut connection, &api, Some(&factures), "Une facture");
    request(&mut connection, &api, None, "Santé");
    collection(&mut connection, "Notifications");

    assert_eq!(
        read(&mut connection),
        vec![
            "API Paiements[Auth[Login] Factures[Lister les factures Une facture] Santé]",
            "Notifications[]",
        ]
    );
}

#[test]
fn names_and_documents_are_sealed_and_the_method_is_not() {
    use devnotes_lib::db::schema::http_requests;
    use diesel::prelude::*;

    let mut connection = open_in_memory().unwrap();
    let api = collection(&mut connection, "API");
    let id = request(&mut connection, &api, None, "Login");

    let (name, method, document): (String, String, String) = http_requests::table
        .find(&id)
        .select((
            http_requests::name,
            http_requests::method,
            http_requests::document,
        ))
        .first(connection.db())
        .unwrap();
    assert_ne!(name, "Login");
    assert!(!document.contains("api.exemple.fr"));
    assert_eq!(method, "GET");

    let opened = store::get_request(&mut connection, &id).unwrap();
    assert_eq!(opened.name, "Login");
    assert_eq!(opened.document.url, "https://api.exemple.fr/Login");
}

#[test]
fn a_saved_request_keeps_what_the_patch_left_out_and_moves_its_instant() {
    let mut connection = open_in_memory().unwrap();
    let api = collection(&mut connection, "API");
    let id = request(&mut connection, &api, None, "Login");

    let patch = HttpRequestPatch {
        method: Some(HttpMethod::Post),
        ..HttpRequestPatch::default()
    };
    let saved =
        store::save_request(&mut connection, &id, &patch, Some("Se connecter"), t1()).unwrap();

    assert_eq!(
        (saved.name.as_str(), saved.method),
        ("Se connecter", HttpMethod::Post)
    );
    assert_eq!(saved.document.url, "https://api.exemple.fr/Login");
    assert_eq!((saved.created_at, saved.updated_at), (t0(), t1()));
    assert!(matches!(
        store::save_request(&mut connection, "absent", &patch, None, t1()),
        Err(StorageError::HttpItemNotFound(_))
    ));
}

#[test]
fn a_request_moves_into_a_folder_at_its_rank_and_the_rest_close_up() {
    let mut connection = open_in_memory().unwrap();
    let api = collection(&mut connection, "API");
    let auth = folder(&mut connection, &api, None, "Auth");
    request(&mut connection, &api, Some(&auth), "Login");
    request(&mut connection, &api, Some(&auth), "Logout");
    let refresh = request(&mut connection, &api, None, "Refresh");

    store::move_item(
        &mut connection,
        &item(HttpItemKind::Request, &refresh),
        &place(&api, Some(&auth), 1),
    )
    .unwrap();
    assert_eq!(
        read(&mut connection),
        vec!["API[Auth[Login Refresh Logout]]"]
    );

    store::move_item(
        &mut connection,
        &item(HttpItemKind::Folder, &auth),
        &place(&api, None, 9),
    )
    .unwrap();
    store::move_item(
        &mut connection,
        &item(HttpItemKind::Request, &refresh),
        &place(&api, None, 0),
    )
    .unwrap();
    assert_eq!(
        read(&mut connection),
        vec!["API[Refresh Auth[Login Logout]]"]
    );
}

#[test]
fn a_folder_cannot_move_into_itself_and_takes_its_contents_to_another_collection() {
    let mut connection = open_in_memory().unwrap();
    let api = collection(&mut connection, "API");
    let other = collection(&mut connection, "Autre");
    let outer = folder(&mut connection, &api, None, "Outer");
    let inner = folder(&mut connection, &api, Some(&outer), "Inner");
    request(&mut connection, &api, Some(&inner), "Deep");

    for target in [&outer, &inner] {
        let refused = store::move_item(
            &mut connection,
            &item(HttpItemKind::Folder, &outer),
            &place(&api, Some(target), 0),
        );
        assert!(matches!(refused, Err(StorageError::Invalid(error)) if error.field == "folderId"));
    }

    store::move_item(
        &mut connection,
        &item(HttpItemKind::Folder, &outer),
        &place(&other, None, 0),
    )
    .unwrap();
    assert_eq!(
        read(&mut connection),
        vec!["API[]", "Autre[Outer[Inner[Deep]]]"]
    );
    let moved = store::tree(&mut connection).unwrap();
    let HttpNode::Folder { children, .. } = &moved.collections[1].children[0] else {
        panic!("Outer is a folder");
    };
    let HttpNode::Folder { children, folder } = &children[0] else {
        panic!("Inner is a folder");
    };
    assert_eq!(folder.collection_id, other);
    let HttpNode::Request { request } = &children[0] else {
        panic!("Deep is a request");
    };
    assert_eq!(request.collection_id, other);
}

#[test]
fn a_request_is_refused_a_folder_of_another_collection() {
    let mut connection = open_in_memory().unwrap();
    let api = collection(&mut connection, "API");
    let other = collection(&mut connection, "Autre");
    let elsewhere = folder(&mut connection, &other, None, "Ailleurs");
    let id = request(&mut connection, &api, None, "Login");

    let refused = store::move_item(
        &mut connection,
        &item(HttpItemKind::Request, &id),
        &place(&api, Some(&elsewhere), 0),
    );
    assert!(matches!(refused, Err(StorageError::Invalid(error)) if error.field == "folderId"));
    assert!(matches!(
        store::create_folder(&mut connection, &api, Some(&elsewhere), "X", t0()),
        Err(StorageError::Invalid(_))
    ));
}

#[test]
fn a_deletion_says_first_what_goes_with_it_then_takes_it() {
    let mut connection = open_in_memory().unwrap();
    let api = collection(&mut connection, "API");
    let outer = folder(&mut connection, &api, None, "Outer");
    let inner = folder(&mut connection, &api, Some(&outer), "Inner");
    request(&mut connection, &api, Some(&outer), "A");
    request(&mut connection, &api, Some(&inner), "B");
    request(&mut connection, &api, None, "C");

    let counted = store::contents(&mut connection, &item(HttpItemKind::Folder, &outer)).unwrap();
    assert_eq!((counted.folders, counted.requests), (1, 2));
    let whole = store::contents(&mut connection, &item(HttpItemKind::Collection, &api)).unwrap();
    assert_eq!((whole.folders, whole.requests), (2, 3));

    store::delete(&mut connection, &item(HttpItemKind::Folder, &outer)).unwrap();
    assert_eq!(read(&mut connection), vec!["API[C]"]);

    store::delete(&mut connection, &item(HttpItemKind::Collection, &api)).unwrap();
    assert!(read(&mut connection).is_empty());
    assert!(matches!(
        store::delete(&mut connection, &item(HttpItemKind::Request, "absent")),
        Err(StorageError::HttpItemNotFound(_))
    ));
}

#[test]
fn a_copy_sits_after_its_original_with_everything_it_held() {
    let mut connection = open_in_memory().unwrap();
    let api = collection(&mut connection, "API");
    let auth = folder(&mut connection, &api, None, "Auth");
    let nested = folder(&mut connection, &api, Some(&auth), "Jetons");
    request(&mut connection, &api, Some(&nested), "Refresh");
    let login = request(&mut connection, &api, Some(&auth), "Login");
    request(&mut connection, &api, None, "Santé");
    collection(&mut connection, "Fin");

    store::duplicate(
        &mut connection,
        &item(HttpItemKind::Request, &login),
        "Login (copie)",
        t1(),
    )
    .unwrap();
    store::duplicate(
        &mut connection,
        &item(HttpItemKind::Folder, &auth),
        "Auth (copie)",
        t1(),
    )
    .unwrap();
    let copy = store::duplicate(
        &mut connection,
        &item(HttpItemKind::Collection, &api),
        "API (copie)",
        t1(),
    )
    .unwrap();

    let auth_tree = "Auth[Jetons[Refresh] Login Login (copie)]";
    let copied_auth = "Auth (copie)[Jetons[Refresh] Login Login (copie)]";
    assert_eq!(
        read(&mut connection),
        vec![
            format!("API[{auth_tree} {copied_auth} Santé]"),
            format!("API (copie)[{auth_tree} {copied_auth} Santé]"),
            "Fin[]".to_string(),
        ]
    );
    assert_eq!(copy.kind, HttpItemKind::Collection);
    assert_ne!(copy.id, api);
}

#[test]
fn collections_are_reordered_and_a_rank_past_the_end_is_the_end() {
    let mut connection = open_in_memory().unwrap();
    let first = collection(&mut connection, "Un");
    collection(&mut connection, "Deux");
    collection(&mut connection, "Trois");

    store::reorder_collection(&mut connection, &first, 99).unwrap();
    assert_eq!(read(&mut connection), vec!["Deux[]", "Trois[]", "Un[]"]);

    store::rename(
        &mut connection,
        &item(HttpItemKind::Collection, &first),
        "Premier",
    )
    .unwrap();
    store::reorder_collection(&mut connection, &first, 0).unwrap();
    assert_eq!(
        read(&mut connection),
        vec!["Premier[]", "Deux[]", "Trois[]"]
    );
}

#[test]
fn a_request_inherits_the_nearest_auth_and_every_header_above_it() {
    use devnotes_lib::http::model::{ContainerSettings, KeyValue, RequestAuth};

    let mut connection = open_in_memory().unwrap();
    let api = collection(&mut connection, "API Paiements");
    let factures = folder(&mut connection, &api, None, "Factures");
    let archives = folder(&mut connection, &api, Some(&factures), "Archives");
    let bearer = RequestAuth::Bearer {
        token: "{{accessToken}}".to_string(),
    };
    store::save_settings(
        &mut connection,
        &item(HttpItemKind::Collection, &api),
        &ContainerSettings {
            auth: bearer.clone(),
            headers: vec![KeyValue {
                key: "Accept".to_string(),
                value: "application/json".to_string(),
                ..KeyValue::default()
            }],
            ..ContainerSettings::default()
        },
    )
    .unwrap();

    let inherited = store::inherited(&mut connection, &api, Some(&archives)).unwrap();
    assert_eq!(inherited.auth, bearer);
    assert_eq!(
        inherited.auth_from.map(|from| (from.kind, from.name)),
        Some((HttpItemKind::Collection, "API Paiements".to_string()))
    );
    assert_eq!(inherited.headers[0].header.key, "Accept");

    let saved = store::settings(&mut connection, &item(HttpItemKind::Collection, &api)).unwrap();
    assert_eq!(saved.auth, bearer);
    assert!(matches!(
        store::settings(&mut connection, &item(HttpItemKind::Request, "r")),
        Err(StorageError::Invalid(_))
    ));
}

#[test]
fn a_body_and_an_auth_are_kept_in_the_sealed_document() {
    use devnotes_lib::http::model::{KeyPlace, RequestAuth, RequestBody};

    let mut connection = open_in_memory().unwrap();
    let api = collection(&mut connection, "API");
    let id = request(&mut connection, &api, None, "Payer");
    let document = RequestDocument {
        url: "/pay".to_string(),
        body: RequestBody::Json {
            text: r#"{"amount": 4900}"#.to_string(),
        },
        auth: RequestAuth::ApiKey {
            name: "X-Api-Key".to_string(),
            value: "secret".to_string(),
            place: KeyPlace::Header,
        },
        ..RequestDocument::default()
    };
    let patch = HttpRequestPatch {
        document: Some(document.clone()),
        ..HttpRequestPatch::default()
    };

    store::save_request(&mut connection, &id, &patch, None, t1()).unwrap();

    assert_eq!(
        store::get_request(&mut connection, &id).unwrap().document,
        document
    );
}

/// A row of a table, every string of it marked.
fn marked(key: &str, value: &str) -> devnotes_lib::http::model::KeyValue {
    devnotes_lib::http::model::KeyValue {
        key: key.to_string(),
        value: value.to_string(),
        description: "zq-row-description".to_string(),
        ..Default::default()
    }
}

/// A collection and a folder with settings of their own, then a request for each kind of
/// auth and both places a key goes. Answers the last request.
fn marked_tree(library: &mut Library) -> String {
    use devnotes_lib::http::model::{ContainerSettings, KeyPlace, RequestAuth, RequestBody};

    let api = collection(library, "zq-collection");
    let payments = folder(library, &api, None, "zq-folder");
    let settings = [
        (
            item(HttpItemKind::Collection, &api),
            RequestAuth::Bearer {
                token: "zq-collection-token".to_string(),
            },
        ),
        (
            item(HttpItemKind::Folder, &payments),
            RequestAuth::Basic {
                username: "zq-folder-user".to_string(),
                password: "zq-folder-password".to_string(),
            },
        ),
    ];
    for (container, auth) in settings {
        let own = ContainerSettings {
            auth,
            headers: vec![marked("X-Zq-Container", "zq-container-header")],
            ..ContainerSettings::default()
        };
        store::save_settings(library, &container, &own).unwrap();
    }

    let auths = [
        RequestAuth::Basic {
            username: "zq-basic-user".to_string(),
            password: "zq-basic-password".to_string(),
        },
        RequestAuth::Bearer {
            token: "zq-bearer-token".to_string(),
        },
        RequestAuth::ApiKey {
            name: "X-Zq-Key".to_string(),
            value: "zq-header-key".to_string(),
            place: KeyPlace::Header,
        },
        RequestAuth::ApiKey {
            name: "zq-key-name".to_string(),
            value: "zq-query-key".to_string(),
            place: KeyPlace::Query,
        },
    ];
    let mut last = String::new();
    for (index, auth) in auths.into_iter().enumerate() {
        let draft = HttpRequestDraft {
            collection_id: api.clone(),
            folder_id: Some(payments.clone()),
            name: format!("zq-request-{index}"),
            // The two columns left in the clear: nothing in them is what was typed.
            kind: RequestKind::Graphql,
            method: HttpMethod::Patch,
            document: RequestDocument {
                url: "https://zq-host.exemple.fr/v1?zq-param=zq-param-value".to_string(),
                params: vec![marked("zq-param", "zq-param-value")],
                headers: vec![marked("X-Zq-Header", "zq-header-value")],
                body: RequestBody::Json {
                    text: r#"{"password":"zq-body"}"#.to_string(),
                },
                auth,
                description: "zq-description".to_string(),
                ..RequestDocument::default()
            },
        };
        last = store::create_request(library, &draft, "zq-request", t0())
            .unwrap()
            .id;
    }
    last
}

/// A send with its answer. What the mask replaces never reaches the vault: these are the
/// strings it lets through — a parameter typed by hand, a body, what was answered.
fn marked_send(library: &mut Library, request_id: &str) {
    use devnotes_lib::http::history::{self, Sent};
    use devnotes_lib::http::send::{Exchange, SentBody, SentResponse};
    use devnotes_lib::notes::language::Language;

    let exchange = Exchange {
        method: HttpMethod::Options,
        url: "https://zq-sent-host.exemple.fr/login?zq-sent-param=zq-sent-value".to_string(),
        headers: vec![marked("X-Zq-Sent", "zq-sent-header")],
        body: SentBody::Text {
            text: "zq-sent-body".to_string(),
        },
    };
    let sent = Sent {
        kind: RequestKind::Http,
        name: "zq-sent-name".to_string(),
        request_id: Some(request_id.to_string()),
        exchange: exchange.clone(),
        secrets: Vec::new(),
        secret_values: vec![("token".to_string(), "zq-secret-value".to_string())],
        environment: Some(devnotes_lib::http::history::SentEnvironment {
            name: "zq-sent-environment".to_string(),
            colour: devnotes_lib::folders::model::FolderColour::Red,
        }),
        outcome: Ok(SentResponse {
            status: 418,
            reason: "zq-reason".to_string(),
            millis: 12,
            size: 2,
            url: "https://zq-answer-host.exemple.fr/home".to_string(),
            headers: vec![marked("X-Zq-Answer", "zq-answer-header")],
            body: "zq-answer-body".to_string(),
            binary: false,
            cut: false,
            incomplete: false,
            redirects: Vec::new(),
            language: Language::Json,
            pretty: Some("zq-pretty".to_string()),
            cookies: Vec::new(),
            exchange,
            graphql: None,
        }),
    };
    let (summary, record) = history::record_of(&sent);
    history::store::record(library, Some(request_id), &summary, &record, t1()).unwrap();

    // Read back whole before the file is looked at: sealed, not lost.
    let entry = &history::store::list(library).unwrap()[0];
    assert_eq!(entry.method, HttpMethod::Options);
    assert_eq!(entry.status, Some(418));
}

fn marked_cookie(library: &mut Library) {
    use devnotes_lib::http::cookies::{Change, JarCookie, store as jar};

    let cookie = JarCookie {
        name: "zq-cookie-name".to_string(),
        value: "zq-cookie-value".to_string(),
        domain: "zq-cookie-host.exemple.fr".to_string(),
        host_only: true,
        path: "/zq-cookie-path".to_string(),
        expires: None,
        secure: true,
        http_only: true,
        same_site: None,
        created: None,
    };
    jar::apply(library, &[Change::Set(cookie)], t1()).unwrap();
    assert_eq!(jar::count(library).unwrap(), 1);
}

fn marked_environments(library: &mut Library) {
    use devnotes_lib::http::environments::{
        EnvironmentPatch, Variable, VariableKind, store as sets,
    };

    let variable = |kind: VariableKind| Variable {
        name: "zq-variable-name".to_string(),
        value: "zq-variable-value".to_string(),
        kind,
        ..Variable::default()
    };
    let staging = sets::create(library, "zq-environment", t0()).unwrap();
    sets::save(
        library,
        &staging.id,
        &EnvironmentPatch {
            protected: Some(true),
            variables: Some(vec![variable(VariableKind::Secret)]),
            ..EnvironmentPatch::default()
        },
        None,
    )
    .unwrap();
    sets::save_globals(library, &[variable(VariableKind::Text)], t0()).unwrap();
    assert_eq!(sets::list(library).unwrap().environments.len(), 1);
}

/// The notes' twin: what a reader of the file alone learns of the HTTP side. Every string typed
/// or answered carries `zq-`, so one that leaks is named by the assertion that finds it.
#[test]
fn nothing_typed_or_answered_is_readable_in_the_file_it_was_written_to() {
    use devnotes_lib::db;

    let scratch = tempfile::tempdir().unwrap();
    let path = scratch.path().join("sealed.sqlite3");
    {
        let mut library = db::open(&path, db::test_vault().unwrap()).unwrap();
        let last = marked_tree(&mut library);
        marked_send(&mut library, &last);
        marked_cookie(&mut library);
        marked_environments(&mut library);
    }

    let raw = std::fs::read(&path).unwrap();
    let haystack = String::from_utf8_lossy(&raw);

    let leaked: Vec<String> = raw
        .windows(3)
        .enumerate()
        .filter(|(_, window)| *window == b"zq-")
        .map(|(at, _)| String::from_utf8_lossy(&raw[at..(at + 24).min(raw.len())]).into_owned())
        .collect();
    assert!(leaked.is_empty(), "in the clear: {leaked:?}");
    // The method of a send is in its sealed summary: no request here is an `OPTIONS`.
    assert!(
        !haystack.contains("OPTIONS"),
        "the method of a send is in the clear"
    );
    // A request's own kind and method are columns, deliberately: a reader learns that four
    // GraphQL `PATCH`es are filed in a folder of a collection, and when. This asserts the
    // decision rather than an accident.
    assert!(haystack.contains("PATCH"));
    assert!(haystack.contains("graphql"));
}

#[test]
fn the_palette_finds_a_request_by_its_sealed_url_and_says_where_it_is_filed() {
    let mut connection = open_in_memory().unwrap();
    let api = collection(&mut connection, "API Paiements");
    let factures = folder(&mut connection, &api, None, "Factures");
    let archives = folder(&mut connection, &api, Some(&factures), "Archives");
    request(&mut connection, &api, Some(&archives), "Ancienne facture");
    request(&mut connection, &api, None, "Santé");

    let hits = search::rank(
        store::search_candidates(&mut connection).unwrap(),
        "exemple.fr/ancienne",
    );

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].name, "Ancienne facture");
    assert_eq!(hits[0].url, "https://api.exemple.fr/Ancienne facture");
    assert_eq!(hits[0].place, ["API Paiements", "Factures", "Archives"]);
    let everything = search::rank(store::search_candidates(&mut connection).unwrap(), "");
    assert_eq!(everything.len(), 2);
}
