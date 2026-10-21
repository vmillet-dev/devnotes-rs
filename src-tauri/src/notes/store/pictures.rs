//! A Dessin's two pictures, sealed, one row per drawing: written with the patch that saves the
//! drawing, read one at a time where it is shown. The rules are in `notes::drawing`.

use diesel::prelude::*;

use crate::db::schema::{drawing_pictures, notes};
use crate::error::StorageError;
use crate::notes::drawing::{DrawingRef, Pictures, sort_drawings, uri_of};
use crate::notes::kind::NoteKind;
use crate::vault::key::Vault;

#[derive(Insertable)]
#[diesel(table_name = drawing_pictures)]
struct PicturesRow<'a> {
    note_id: &'a str,
    thumbnail: Vec<u8>,
    picture: Vec<u8>,
}

/// Replaces whatever the drawing had: the pictures are always taken together.
pub fn save(
    connection: &mut SqliteConnection,
    vault: &Vault,
    note_id: &str,
    pictures: &Pictures,
) -> Result<(), StorageError> {
    diesel::replace_into(drawing_pictures::table)
        .values(PicturesRow {
            note_id,
            thumbnail: vault.seal_bytes(&pictures.thumbnail)?,
            picture: vault.seal_bytes(&pictures.picture)?,
        })
        .execute(connection)?;
    Ok(())
}

pub fn remove(connection: &mut SqliteConnection, note_id: &str) -> Result<(), StorageError> {
    diesel::delete(drawing_pictures::table.find(note_id)).execute(connection)?;
    Ok(())
}

#[derive(Clone, Copy)]
pub enum Which {
    Thumbnail,
    Picture,
}

/// One of a drawing's pictures, opened, as a data URI. `None` for a drawing in the trash, one
/// whose pictures were never taken, or no drawing at all.
pub fn read(
    connection: &mut SqliteConnection,
    vault: &Vault,
    note_id: &str,
    which: Which,
) -> Result<Option<String>, StorageError> {
    let live = drawing_pictures::table
        .inner_join(notes::table)
        .filter(drawing_pictures::note_id.eq(note_id))
        .filter(notes::deleted_at.is_null());
    // Only the one asked for leaves the database: the other is the bigger, or the smaller.
    let sealed: Option<Vec<u8>> = match which {
        Which::Thumbnail => live.select(drawing_pictures::thumbnail).first(connection),
        Which::Picture => live.select(drawing_pictures::picture).first(connection),
    }
    .optional()?;

    sealed
        .map(|bytes| Ok(uri_of(&vault.open_bytes(&bytes)?)))
        .transpose()
}

/// Every drawing out of the trash, by title: only their titles are opened.
pub fn drawings(
    connection: &mut SqliteConnection,
    vault: &Vault,
) -> Result<Vec<DrawingRef>, StorageError> {
    let rows: Vec<(String, String)> = notes::table
        .filter(notes::kind.eq(NoteKind::Drawing.as_str()))
        .filter(notes::deleted_at.is_null())
        .select((notes::id, notes::title))
        .load(connection)?;
    let mut drawings = rows
        .into_iter()
        .map(|(id, title)| {
            Ok(DrawingRef {
                id,
                title: vault.open(&title)?,
            })
        })
        .collect::<Result<Vec<_>, StorageError>>()?;
    sort_drawings(&mut drawings);
    Ok(drawings)
}

/// A duplicate's: the sealed bytes as they are, under the copy's id.
pub fn copy(connection: &mut SqliteConnection, from: &str, to: &str) -> Result<(), StorageError> {
    let Some((thumbnail, picture)) = drawing_pictures::table
        .filter(drawing_pictures::note_id.eq(from))
        .select((drawing_pictures::thumbnail, drawing_pictures::picture))
        .first::<(Vec<u8>, Vec<u8>)>(connection)
        .optional()?
    else {
        return Ok(());
    };

    diesel::insert_into(drawing_pictures::table)
        .values(PicturesRow {
            note_id: to,
            thumbnail,
            picture,
        })
        .execute(connection)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Library;
    use crate::notes::drawing::DrawingPictures;
    use crate::notes::fixtures::{NOW, at, note};
    use crate::notes::kind::NoteKind;
    use crate::notes::model::{Note, NotePatch};
    use crate::notes::store;

    const SCENE: &str = r#"{"version":1,"scene":{"objects":[]}}"#;

    fn png(tag: u8) -> String {
        uri_of(&[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n', tag])
    }

    fn library_with(kind: NoteKind) -> (Library, Note) {
        let mut library = crate::db::open_in_memory().unwrap();
        let space_id = crate::spaces::store::create(&mut library, "Perso")
            .unwrap()
            .id;
        let drawing = Note {
            space_id,
            kind,
            content: SCENE.to_string(),
            ..note()
        };
        store::insert_imported(&mut library, &drawing).unwrap();
        (library, drawing)
    }

    fn save(library: &mut Library, id: &str, thumbnail: u8, picture: u8) {
        let sent = DrawingPictures {
            thumbnail: png(thumbnail),
            picture: png(picture),
        };
        let patch = NotePatch {
            content: Some(SCENE.to_string()),
            pictures: Some(sent.clone()),
            ..NotePatch::default()
        };
        store::update_drawn(
            library,
            id,
            &patch,
            Some(sent.decoded().unwrap().as_ref()),
            at(NOW),
        )
        .unwrap();
    }

    fn shown(library: &mut Library, id: &str, which: Which) -> Option<String> {
        let (connection, vault) = library.split();
        read(connection, vault, id, which).unwrap()
    }

    #[test]
    fn a_drawing_saved_gives_each_picture_alone_and_the_last_taken() {
        let (mut library, drawing) = library_with(NoteKind::Drawing);
        assert_eq!(shown(&mut library, &drawing.id, Which::Thumbnail), None);

        save(&mut library, &drawing.id, 1, 2);
        save(&mut library, &drawing.id, 3, 4);

        assert_eq!(
            shown(&mut library, &drawing.id, Which::Thumbnail),
            Some(png(3))
        );
        assert_eq!(
            shown(&mut library, &drawing.id, Which::Picture),
            Some(png(4))
        );
    }

    /// Sealed like a body: what the file holds is no PNG.
    #[test]
    fn the_pictures_are_sealed_in_the_file() {
        let (mut library, drawing) = library_with(NoteKind::Drawing);
        save(&mut library, &drawing.id, 1, 2);

        let (connection, _) = library.split();
        let stored: Vec<u8> = drawing_pictures::table
            .select(drawing_pictures::thumbnail)
            .first(connection)
            .unwrap();
        assert!(!stored.windows(4).any(|bytes| bytes == b"PNG\r"));
    }

    #[test]
    fn a_drawing_emptied_has_no_picture_any_more() {
        let (mut library, drawing) = library_with(NoteKind::Drawing);
        save(&mut library, &drawing.id, 1, 2);

        let patch = NotePatch::default();
        store::update_drawn(&mut library, &drawing.id, &patch, Some(None), at(NOW)).unwrap();

        assert_eq!(shown(&mut library, &drawing.id, Which::Thumbnail), None);
    }

    #[test]
    fn the_drawings_a_note_can_embed_are_named_and_the_trashed_left_out() {
        let (mut library, drawing) = library_with(NoteKind::Drawing);
        let trashed = Note {
            id: "trashed".to_string(),
            space_id: drawing.space_id.clone(),
            kind: NoteKind::Drawing,
            ..note()
        };
        store::insert_imported(&mut library, &trashed).unwrap();
        store::trash::trash(&mut library, "trashed", at(NOW)).unwrap();

        let (connection, vault) = library.split();
        let listed = drawings(connection, vault).unwrap();

        assert_eq!(
            listed,
            [DrawingRef {
                id: drawing.id.clone(),
                title: drawing.title.clone()
            }]
        );
    }

    #[test]
    fn what_is_no_drawing_keeps_no_picture() {
        let (mut library, snippet) = library_with(NoteKind::Snippet);

        save(&mut library, &snippet.id, 1, 2);

        assert_eq!(shown(&mut library, &snippet.id, Which::Picture), None);
    }

    #[test]
    fn a_drawing_in_the_trash_shows_nothing_and_a_purge_takes_its_pictures() {
        let (mut library, drawing) = library_with(NoteKind::Drawing);
        save(&mut library, &drawing.id, 1, 2);

        store::trash::trash(&mut library, &drawing.id, at(NOW)).unwrap();
        assert_eq!(shown(&mut library, &drawing.id, Which::Picture), None);

        store::trash::purge(&mut library, std::slice::from_ref(&drawing.id)).unwrap();
        let (connection, _) = library.split();
        let left: i64 = drawing_pictures::table
            .count()
            .get_result(connection)
            .unwrap();
        assert_eq!(left, 0);
    }

    #[test]
    fn a_copy_takes_the_pictures_under_its_own_id() {
        let (mut library, drawing) = library_with(NoteKind::Drawing);
        save(&mut library, &drawing.id, 1, 2);

        let twin =
            crate::notes::duplicate::duplicate(&mut library, &drawing.id, "Copie".into(), at(NOW))
                .unwrap();

        assert_eq!(
            shown(&mut library, &twin.id, Which::Thumbnail),
            Some(png(1))
        );
        let (connection, _) = library.split();
        assert!(copy(connection, "nothing", "else").is_ok());
    }
}
