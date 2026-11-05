//! A collection's or a folder's settings while their dialog is open: read, edited whole, and
//! written on « Save ». Every request under it inherits them.

use engine::http::model::{ContainerSettings, HttpItem, HttpItemKind};
use engine::http::settings::Inherited;
use i18n::keys;

use super::tree::RailRow;
use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;

/// What the dialog is open on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    pub item: HttpItem,
    pub name: String,
    pub collection_id: String,
    pub folder_id: Option<String>,
}

impl Container {
    pub fn of(row: &RailRow) -> Self {
        Self {
            item: HttpItem {
                kind: row.kind,
                id: row.id.clone(),
            },
            name: row.name.clone(),
            collection_id: row.collection_id.clone(),
            folder_id: row.folder_id.clone(),
        }
    }
}

pub struct Form {
    pub container: Container,
    /// `None` until read.
    pub settings: Option<ContainerSettings>,
    /// What a folder's « Inherited » takes; a collection has nothing above it.
    pub above: Option<Inherited>,
    reading: Option<Pending<ContainerSettings>>,
    reading_above: Option<Pending<Inherited>>,
    saving: Option<Pending<()>>,
}

impl Form {
    pub fn open(backend: &Backend, container: Container) -> Self {
        let item = container.item.clone();
        let reading_above = (item.kind == HttpItemKind::Folder).then(|| {
            let collection = container.collection_id.clone();
            // From its parent: what it sets itself is not what it inherits.
            api::http::inherited(backend, collection, container.folder_id.clone())
        });
        Self {
            reading: Some(api::http::settings(backend, item)),
            reading_above,
            container,
            settings: None,
            above: None,
            saving: None,
        }
    }

    pub fn save(&mut self, backend: &Backend) {
        if let Some(settings) = self.settings.clone() {
            let item = self.container.item.clone();
            self.saving = Some(api::http::save_settings(backend, item, settings));
        }
    }

    /// Once a frame. `Some(true)` once written, `Some(false)` when it could not be read: either
    /// way the dialog is done.
    pub fn update(&mut self, errors: &mut ErrorNotifier) -> Option<bool> {
        if let Some(pending) = &mut self.reading
            && let Some(answer) = pending.poll()
        {
            self.reading = None;
            match answer {
                Ok(settings) => self.settings = Some(settings),
                Err(error) => {
                    errors.report(keys::http::FAILED, &error);
                    return Some(false);
                }
            }
        }
        if let Some(pending) = &mut self.reading_above
            && let Some(answer) = pending.poll()
        {
            self.reading_above = None;
            match answer {
                Ok(above) => self.above = Some(above),
                Err(error) => errors.report(keys::http::FAILED, &error),
            }
        }
        let answer = self.saving.as_mut()?.poll()?;
        self.saving = None;
        match answer {
            Ok(()) => Some(true),
            Err(error) => {
                errors.report(keys::http::FAILED, &error);
                None
            }
        }
    }
}
