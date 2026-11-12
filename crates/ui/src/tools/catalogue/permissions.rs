//! Three ways into one mode — octal, letters, boxes — each written back from Rust's answer.

use egui::{Ui, Vec2};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::permissions::{
    Class, ClassRights, Mode, ModeReading, PermissionsAnswer, PermissionsRequest, UmaskReading,
};
use i18n::{Language, Message, keys};
use serde_json::json;

use crate::api;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{
    ResultRow, card, check, columns, hint, label, named, paragraph, problem, result_rows,
    section_title,
};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{TextField, address};

const CLASSES: [Class; 3] = [Class::Owner, Class::Group, Class::Others];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Octal,
    Symbolic,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Right {
    Read,
    Write,
    Execute,
    Special,
}

pub(super) struct PermissionsTool {
    /// What was typed, in the field it was typed in: the other shows Rust's reading of it.
    field: Field,
    text: String,
    umask: String,
    answer: Live<PermissionsRequest, PermissionsAnswer>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::new(PermissionsTool {
        field: Field::Octal,
        text: String::new(),
        umask: "022".to_owned(),
        answer: Live::default(),
    })
}

fn class_id(class: Class) -> &'static str {
    match class {
        Class::Owner => "owner",
        Class::Group => "group",
        Class::Others => "others",
    }
}

/// The bit a box flips.
fn mask_of(class: Class, right: Right) -> u32 {
    let shift = match class {
        Class::Owner => 6,
        Class::Group => 3,
        Class::Others => 0,
    };
    match (right, class) {
        (Right::Read, _) => 4 << shift,
        (Right::Write, _) => 2 << shift,
        (Right::Execute, _) => 1 << shift,
        (Right::Special, Class::Owner) => 0o4000,
        (Right::Special, Class::Group) => 0o2000,
        (Right::Special, Class::Others) => 0o1000,
    }
}

fn held(rights: Option<&ClassRights>, right: Right) -> bool {
    rights.is_some_and(|it| match right {
        Right::Read => it.read,
        Right::Write => it.write,
        Right::Execute => it.execute,
        Right::Special => it.special,
    })
}

fn say(language: Language, path: &str) -> String {
    i18n::Key::find(path).map_or_else(String::new, |key| i18n::translate(language, key, &[]))
}

/// « read, write, execute ».
fn rights_words(rights: &ClassRights, language: Language) -> String {
    let granted: Vec<String> = [
        (rights.read, "read"),
        (rights.write, "write"),
        (rights.execute, "execute"),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .map(|(_, id)| say(language, &format!("tools.permissions.words.{id}")))
    .collect();
    if granted.is_empty() {
        i18n::translate(language, keys::tools::permissions::words::NONE, &[])
    } else {
        granted.join(", ")
    }
}

impl PermissionsTool {
    fn mode(&self) -> Option<&Mode> {
        match &self.answer.value()?.mode {
            ModeReading::Read { mode, .. } => Some(mode),
            _ => None,
        }
    }

    /// A field shows what was typed in it, or else Rust's writing of the mode.
    fn shown(&self, field: Field) -> String {
        if self.field == field {
            return self.text.clone();
        }
        self.mode().map_or_else(String::new, |mode| match field {
            Field::Octal => mode.octal.clone(),
            Field::Symbolic => mode.symbolic.clone(),
        })
    }

    /// One box flips one bit; the mode is written back as octal, which Rust reads again.
    fn toggle(&mut self, class: Class, right: Right) {
        let bits = self.mode().map_or(0, |mode| mode.bits);
        let next = bits ^ mask_of(class, right);
        self.field = Field::Octal;
        self.text = if next > 0o777 {
            format!("{next:04o}")
        } else {
            format!("{next:03o}")
        };
    }

    fn input(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        self.fields(ui, language);
        self.about_the_mode(ui, language);
        self.grid(ui, language);
        self.warnings(ui, language);
        self.umask_section(ui, language);
    }

    fn fields(&mut self, ui: &mut Ui, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            for (field, width, name, hint_key, id) in [
                (
                    Field::Octal,
                    110.0,
                    keys::tools::permissions::OCTAL,
                    keys::tools::permissions::OCTAL_PLACEHOLDER,
                    "permissions-octal",
                ),
                (
                    Field::Symbolic,
                    ui.available_width() - 120.0,
                    keys::tools::permissions::SYMBOLIC,
                    keys::tools::permissions::SYMBOLIC_PLACEHOLDER,
                    "permissions-symbolic",
                ),
            ] {
                ui.allocate_ui(Vec2::new(width, 0.0), |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(width);
                        let name = tr(name);
                        label(ui, &name);
                        let mut shown = self.shown(field);
                        let edited = TextField::new(&mut shown, &name, id)
                            .hint(&tr(hint_key))
                            .show(ui);
                        if edited.changed() {
                            self.field = field;
                            self.text = shown;
                        }
                    });
                });
            }
        });
    }

    fn about_the_mode(&self, ui: &mut Ui, language: Language) {
        let Some(answer) = self.answer.value() else {
            return;
        };
        match &answer.mode {
            ModeReading::Refused {
                problem: reason,
                at,
            } => {
                let words = refused(*reason, *at, language);
                problem(ui, &words, "permissions-problem");
            }
            ModeReading::Read {
                file_type: Some(kind),
                ..
            } => {
                let kind = named(language, "tools.permissions.types", &json!(kind));
                let words = Message::new(keys::tools::permissions::FILE_TYPE)
                    .with("type", kind)
                    .text(language);
                let said = hint(ui, &words);
                address(ui, &said, "permissions-type");
            }
            _ => {}
        }
    }

    fn grid(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::permissions::{classes, rights};
        let tr = |key| i18n::translate(language, key, &[]);
        let names = [
            (Right::Read, tr(rights::READ)),
            (Right::Write, tr(rights::WRITE)),
            (Right::Execute, tr(rights::EXECUTE)),
        ];
        let mut flipped = None;
        let held_now: Vec<Option<ClassRights>> = CLASSES
            .iter()
            .map(|class| {
                let classes = self.mode().map(|mode| mode.classes.as_slice());
                classes
                    .and_then(|it| it.iter().find(|row| row.class == *class))
                    .cloned()
            })
            .collect();
        for (class, rights_held) in CLASSES.iter().zip(&held_now) {
            let id = class_id(*class);
            let name = tr(match class {
                Class::Owner => classes::OWNER,
                Class::Group => classes::GROUP,
                Class::Others => classes::OTHERS,
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                ui.allocate_ui(Vec2::new(84.0, 24.0), |ui| {
                    ui.set_width(84.0);
                    label(ui, &name);
                });
                for (right, right_name) in &names {
                    let mut on = held(rights_held.as_ref(), *right);
                    let right_id = match right {
                        Right::Read => "read",
                        Right::Write => "write",
                        _ => "execute",
                    };
                    let box_id = format!("permissions-{id}-{right_id}");
                    if check(ui, &mut on, right_name, &box_id).clicked() {
                        flipped = Some((*class, *right));
                    }
                }
                let mut on = held(rights_held.as_ref(), Right::Special);
                let special = say(language, &format!("tools.permissions.specials.{id}"));
                let box_id = format!("permissions-{id}-special");
                if check(ui, &mut on, &special, &box_id).clicked() {
                    flipped = Some((*class, Right::Special));
                }
            });
        }
        if let Some((class, right)) = flipped {
            self.toggle(class, right);
        }
    }

    fn warnings(&self, ui: &mut Ui, language: Language) {
        let p = palette(ui.ctx());
        let Some(ModeReading::Read { warnings, .. }) = self.answer.value().map(|it| &it.mode)
        else {
            return;
        };
        for warning in warnings {
            let words = named(language, "tools.permissions.warnings", &json!(warning));
            let said = paragraph(ui, &words, (12.0, 400.0), p.amber_text);
            address(ui, &said, "permissions-warning");
        }
    }

    fn umask_section(&mut self, ui: &mut Ui, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        let name = tr(keys::tools::permissions::UMASK);
        ui.add_space(8.0);
        section_title(ui, &name);
        TextField::new(&mut self.umask, &name, "permissions-umask")
            .hint(&tr(keys::tools::permissions::UMASK_PLACEHOLDER))
            .show(ui);
        match self.answer.value().map(|it| &it.umask) {
            Some(UmaskReading::Read { file, directory }) => {
                let file_name = tr(keys::tools::permissions::NEW_FILE);
                let directory_name = tr(keys::tools::permissions::NEW_DIRECTORY);
                let file_value = format!("{} · {}", file.octal, file.symbolic);
                let directory_value = format!("{} · {}", directory.octal, directory.symbolic);
                card(ui, "permissions-umask-result", |ui| {
                    let rows = [
                        ResultRow::new(&file_name, &file_value).reading(),
                        ResultRow::new(&directory_name, &directory_value).reading(),
                    ];
                    result_rows(ui, &rows, language);
                });
            }
            Some(UmaskReading::Refused {
                problem: reason,
                at,
            }) => {
                let words = refused(*reason, *at, language);
                problem(ui, &words, "permissions-umask-problem");
            }
            _ => {}
        }
    }

    fn output(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(keys::tools::permissions::COMMANDS));
        let Some(mode) = self.mode() else {
            hint(ui, &tr(keys::tools::permissions::EMPTY));
            return;
        };
        let file = tr(keys::tools::permissions::FILE);
        let octal = format!("chmod {} {file}", mode.octal);
        let symbolic = format!("chmod {} {file}", mode.chmod_symbolic);
        card(ui, "permissions-commands", |ui| {
            let rows = [
                ResultRow::new("chmod", &octal),
                ResultRow::new("chmod", &symbolic),
            ];
            result_rows(ui, &rows, language);
        });
        section_title(ui, &tr(keys::tools::permissions::CAN));
        let lines: Vec<(String, String)> = mode
            .classes
            .iter()
            .map(|rights| {
                let id = class_id(rights.class);
                let name = say(language, &format!("tools.permissions.classes.{id}"));
                let mut text = rights_words(rights, language);
                if rights.special {
                    let special = say(language, &format!("tools.permissions.specialWords.{id}"));
                    text = format!("{text} — {special}");
                }
                (name, text)
            })
            .collect();
        card(ui, "permissions-words", |ui| {
            let rows: Vec<_> = lines
                .iter()
                .map(|(name, text)| ResultRow::new(name, text).reading())
                .collect();
            result_rows(ui, &rows, language);
        });
        hint(ui, &tr(keys::tools::permissions::DIRECTORY_EXECUTE));
    }
}

fn refused(reason: engine::tools::permissions::ModeProblem, at: u32, language: Language) -> String {
    let id = json!(reason);
    let path = format!(
        "tools.permissions.problems.{}",
        id.as_str().unwrap_or_default()
    );
    i18n::Key::find(&path).map_or_else(String::new, |key| {
        Message::new(key).with("at", at).text(language)
    })
}

impl Tool for PermissionsTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        let empty = self.text.trim().is_empty() && self.umask.trim().is_empty();
        let request = (!empty).then(|| PermissionsRequest {
            mode: self.text.clone(),
            umask: self.umask.clone(),
        });
        self.answer
            .update(turn, request, api::tools::describe_permissions);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::input, Self::output));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let mode = self.mode()?;
        let file = i18n::translate(language, keys::tools::permissions::FILE, &[]);
        let lines = [
            format!("chmod {} {file}", mode.octal),
            format!("chmod {} {file}", mode.chmod_symbolic),
            format!("# {}", mode.symbolic),
        ];
        Some(ToolResult {
            title: Message::new(keys::tools::permissions::NOTE_TITLE)
                .with("octal", mode.octal.as_str())
                .text(language),
            language: NoteLanguage::Sh,
            content: lines.join("\n"),
            warning: None,
        })
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        self.field = Field::Octal;
        "754".clone_into(&mut self.text);
        "027".clone_into(&mut self.umask);
    }

    fn clear(&mut self) {
        self.field = Field::Octal;
        self.text.clear();
        self.umask.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_flips_its_one_bit() {
        assert_eq!(mask_of(Class::Owner, Right::Read), 0o400);
        assert_eq!(mask_of(Class::Group, Right::Write), 0o020);
        assert_eq!(mask_of(Class::Others, Right::Execute), 0o001);
        assert_eq!(mask_of(Class::Group, Right::Special), 0o2000);
    }
}
