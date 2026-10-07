//! The HTTP collections as data: a tree of collections, folders and requests, sealed, ordered
//! under each parent, moved, copied and deleted with what they hold.

use devnotes_lib::db::{Library, open_in_memory};
use devnotes_lib::error::StorageError;
use devnotes_lib::http::model::{
    HttpItem, HttpItemKind, HttpMethod, HttpNode, HttpPlace, HttpRequestDraft, HttpRequestPatch,
    HttpTree, RequestDocument, RequestKind,
};
use devnotes_lib::http::store;

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
