//! A block read and described by Rust; the page groups its digits in the reader's language.

use egui::{Ui, Vec2};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::cidr::forms::ReverseDns;
use engine::tools::cidr::masks::MaskRow;
use engine::tools::cidr::plan::{CidrPlan, PlanLine};
use engine::tools::cidr::summary::{CidrSummary, FamilySummary};
use engine::tools::cidr::{
    CidrAnswer, CidrBlock, CidrMembership, CidrRequest, CidrSplit, IpFamily,
};
use i18n::{Language, Message, keys};
use serde_json::json;

use crate::api;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{
    ResultRow, TextColumn, card, columns, hint, named, paragraph, problem, result_rows,
    section_head, section_title, text_area, text_table,
};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{Button, Segmented, TextField, address};

/// What the masks' columns need to show what they hold.
const MASKS_WIDTH: f32 = 470.0;

const SAMPLE_PLAN: &str = "Bureaux 500\nWi-Fi invités 120\nServeurs 60\nImprimantes 12\nLien WAN 2";
const SAMPLE_LIST: &str =
    "10.24.8.0/24\n10.24.9.0/24\n10.24.10.0/23\n10.24.12.0/24\n10.24.13.5 - 10.24.13.20";

#[derive(Default)]
pub(super) struct CidrTool {
    network: String,
    family: Option<IpFamily>,
    member: String,
    split: Option<u8>,
    plan: String,
    list: String,
    answer: Live<CidrRequest, CidrAnswer>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::<CidrTool>::default()
}

fn family_id(family: IpFamily) -> &'static str {
    match family {
        IpFamily::V4 => "v4",
        IpFamily::V6 => "v6",
    }
}

fn say(language: Language, path: &str) -> String {
    i18n::Key::find(path).map_or_else(String::new, |key| i18n::translate(language, key, &[]))
}

/// Digits from Rust, grouped as the reader's language groups them: an IPv6 count passes 2⁵³.
fn grouped(digits: &str, language: Language) -> String {
    i18n::format::exact(digits, (0, 0), false, true, language)
}

fn plural(digits: &str) -> u32 {
    if digits == "1" { 1 } else { 2 }
}

fn hosts_of(block: &CidrBlock, language: Language) -> String {
    let path = format!("tools.cidr.usableOf.{}", family_id(block.family));
    i18n::Key::find(&path).map_or_else(String::new, |key| {
        Message::new(key)
            .with("usable", grouped(&block.usable, language))
            .with("total", grouped(&block.addresses, language))
            .with("count", plural(&block.addresses))
            .text(language)
    })
}

/// IPv6 counts read as a power of two: 2¹¹² is a size, its 34 digits are not.
fn mask_addresses(row: &MaskRow, family: IpFamily, language: Language) -> String {
    match family {
        IpFamily::V4 => grouped(&row.addresses, language),
        IpFamily::V6 => format!("2^{}", row.host_bits),
    }
}

/// The card's rows, in its order; IPv6 has neither mask nor broadcast.
fn fact_rows(block: &CidrBlock) -> Vec<(&'static str, String)> {
    let network = match block.family {
        IpFamily::V4 => block.network.clone(),
        IpFamily::V6 => block.cidr.clone(),
    };
    [
        ("network", Some(network)),
        ("mask", block.mask.clone()),
        ("inverseMask", block.inverse_mask.clone()),
        ("first", Some(block.first.clone())),
        ("last", Some(block.last.clone())),
        ("broadcast", block.broadcast.clone()),
    ]
    .into_iter()
    .filter_map(|(id, value)| Some((id, value?)))
    .collect()
}

fn reverse_text(reverse: &ReverseDns, language: Language) -> (String, bool) {
    use keys::tools::cidr::reverse as words;
    match reverse {
        ReverseDns::Record { name } => (name.clone(), true),
        ReverseDns::Zones {
            first, count: 1, ..
        } => (first.clone(), true),
        ReverseDns::Zones { first, last, count } => {
            let text = Message::new(words::ZONES)
                .with("count", *count)
                .with("first", first.as_str())
                .with("last", last.as_str())
                .text(language);
            (text, false)
        }
        ReverseDns::Classless { zone } => {
            let text = Message::new(words::CLASSLESS)
                .with("zone", zone.as_str())
                .text(language);
            (text, false)
        }
    }
}

fn written_card(ui: &mut Ui, block: &CidrBlock, language: Language) {
    let family = family_id(block.family);
    let name = |id: &str| say(language, &format!("tools.cidr.written.{id}.{family}"));
    let (reverse, pasted) = reverse_text(&block.forms.reverse, language);
    let names = (name("integer"), name("expanded"), name("reverse"));
    card(ui, "cidr-written", |ui| {
        let reverse_row = ResultRow::new(&names.2, &reverse);
        let rows = [
            ResultRow::new(&names.0, &block.forms.integer),
            ResultRow::new(&names.1, &block.forms.expanded),
            if pasted {
                reverse_row
            } else {
                reverse_row.reading()
            },
        ];
        result_rows(ui, &rows, language);
    });
}

fn binary_card(ui: &mut Ui, block: &CidrBlock, language: Language) {
    let p = palette(ui.ctx());
    let words = Message::new(keys::tools::cidr::BITS)
        .with("network", u32::from(block.prefix))
        .with("hosts", u32::from(block.host_bits))
        .text(language);
    hint(ui, &words);
    let mut job = egui::text::LayoutJob::default();
    let format = |ink| egui::TextFormat::simple(egui::FontId::monospace(12.0), ink);
    job.append(&block.binary_network, 0.0, format(p.amber_text));
    job.append(&block.binary_host, 0.0, format(p.text_2));
    job.wrap.max_width = ui.available_width();
    let shown = ui.add(egui::Label::new(job));
    address(ui, &shown, "cidr-binary");
    let path = format!(
        "tools.cidr.kinds.{}",
        json!(block.kind).as_str().unwrap_or_default()
    );
    let words = i18n::Key::find(&path).map_or_else(String::new, |key| {
        Message::new(key)
            .with("range", block.range.as_deref().unwrap_or_default())
            .text(language)
    });
    let caveat = i18n::translate(language, keys::tools::cidr::CAVEAT, &[]);
    let said = hint(ui, &format!("{words} {caveat}"));
    address(ui, &said, "cidr-kind");
}

fn subnets(ui: &mut Ui, split: &CidrSplit, family: IpFamily, language: Language) {
    let path = format!("tools.cidr.subnetHosts.{}", family_id(family));
    let shown: Vec<(String, &str)> = split
        .subnets
        .iter()
        .map(|subnet| {
            let hosts = i18n::Key::find(&path).map_or_else(String::new, |key| {
                Message::new(key)
                    .with("count", plural(&subnet.usable))
                    .with("hosts", grouped(&subnet.usable, language))
                    .text(language)
            });
            (hosts, subnet.cidr.as_str())
        })
        .collect();
    card(ui, "cidr-subnets", |ui| {
        let rows: Vec<_> = shown
            .iter()
            .map(|(hosts, cidr)| ResultRow::new(cidr, hosts).copied(cidr))
            .collect();
        result_rows(ui, &rows, language);
    });
    if split.more != "0" {
        let words = Message::new(keys::tools::cidr::MORE)
            .with("count", 2_u32)
            .with("more", grouped(&split.more, language))
            .text(language);
        let said = hint(ui, &words);
        address(ui, &said, "cidr-more");
    }
}

fn plan_table(ui: &mut Ui, plan: &CidrPlan, language: Language) {
    use keys::tools::cidr::plan as words;
    let tr = |key| i18n::translate(language, key, &[]);
    let unnamed = tr(words::UNNAMED);
    let name_of = |name: &str| {
        if name.is_empty() {
            unnamed.clone()
        } else {
            name.to_owned()
        }
    };
    let rows: Vec<Vec<String>> = plan
        .lines
        .iter()
        .map(|line| match line {
            PlanLine::Placed {
                name,
                hosts,
                cidr,
                usable,
                first,
                last,
            } => vec![
                name_of(name),
                grouped(hosts, language),
                cidr.clone(),
                grouped(usable, language),
                format!("{first} – {last}"),
            ],
            PlanLine::NoRoom { name, hosts } => vec![
                name_of(name),
                grouped(hosts, language),
                tr(words::NO_ROOM),
                String::new(),
                String::new(),
            ],
            PlanLine::Unreadable { text } => vec![
                Message::new(words::UNREADABLE)
                    .with("text", text.as_str())
                    .text(language),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
        })
        .collect();
    let headers = [
        TextColumn::new(tr(words::NEED), 0.24),
        TextColumn::new(tr(words::ASKED), 0.12),
        TextColumn::new(tr(words::SUBNET), 0.2),
        TextColumn::new(tr(words::USABLE), 0.12),
        TextColumn::new(tr(words::RANGE), 0.32),
    ];
    text_table(ui, "cidr-plan-lines", &headers, &rows, None);
    let said = if plan.free.is_empty() {
        tr(words::FULL)
    } else {
        format!("{} {}", tr(words::FREE), plan.free.join(" "))
    };
    let said = hint(ui, &said);
    address(ui, &said, "cidr-plan-free");
}

fn family_card(ui: &mut Ui, family: &FamilySummary, language: Language) {
    use keys::tools::cidr::list as words;
    let tr = |key| i18n::translate(language, key, &[]);
    let meta = if family.extra == "0" {
        tr(words::EXACT)
    } else {
        Message::new(words::EXTRA)
            .with("extra", grouped(&family.extra, language))
            .with("count", plural(&family.extra))
            .text(language)
    };
    let count = u32::try_from(family.blocks.len()).unwrap_or(u32::MAX);
    let path = format!("tools.cidr.list.blocks.{}", family_id(family.family));
    let title = i18n::Key::find(&path).map_or_else(String::new, |key| {
        Message::new(key).with("count", count).text(language)
    });
    let blocks = family.blocks.join("\n");
    let supernet = tr(words::SUPERNET);
    card(ui, "cidr-summary", |ui| {
        let row = ResultRow::new(&supernet, &family.supernet).meta(&meta);
        result_rows(ui, &[row], language);
        let row = ResultRow::new(&title, &blocks);
        result_rows(ui, &[row], language);
    });
}

fn summary_of(ui: &mut Ui, summary: &CidrSummary, language: Language) {
    for family in &summary.families {
        family_card(ui, family, language);
    }
    if !summary.unreadable.is_empty() {
        let words = Message::new(keys::tools::cidr::list::UNREADABLE)
            .with("entries", summary.unreadable.join(", "))
            .text(language);
        problem(ui, &words, "cidr-list-unreadable");
    }
}

impl CidrTool {
    fn block(&self) -> Option<&CidrBlock> {
        self.answer.value()?.block.as_ref()
    }

    fn network_area(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::cidr as words;
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let options = [
            (None, tr(words::families::AUTO)),
            (Some(IpFamily::V4), tr(words::families::V4)),
            (Some(IpFamily::V6), tr(words::families::V6)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        let name = tr(words::NETWORK);
        let family = &mut self.family;
        section_head(ui, &name, |ui| {
            Segmented::new(&options, "cidr-family").show(ui, family);
        });
        let field = TextField::new(&mut self.network, &name, "cidr-network")
            .hint(&tr(words::PLACEHOLDER))
            .show(ui);
        if field.changed() {
            self.split = None;
        }
        if let Some(reason) = self.answer.value().and_then(|it| it.problem.as_ref()) {
            let said = named(language, "tools.cidr.problems", &json!(reason));
            problem(ui, &said, "cidr-problem");
        }
        let Some(block) = self.block() else {
            return;
        };
        if block.normalised {
            let words = Message::new(words::NORMALISED)
                .with("cidr", block.cidr.as_str())
                .text(language);
            let said = hint(ui, &words);
            address(ui, &said, "cidr-normalised");
        }
        binary_card(ui, block, language);
        self.member_area(ui, language);
        section_title(ui, &tr(words::WRITTEN_TITLE));
        if let Some(block) = self.block() {
            written_card(ui, block, language);
        }
        self.plan_area(ui, language);
        self.list_area(ui, language);
    }

    fn member_area(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::cidr as words;
        let p = palette(ui.ctx());
        let tr = |key| i18n::translate(language, key, &[]);
        let name = tr(words::MEMBER_TITLE);
        section_title(ui, &name);
        TextField::new(&mut self.member, &name, "cidr-member")
            .hint(&tr(words::MEMBER_PLACEHOLDER))
            .show(ui);
        match self.answer.value().map(|it| &it.membership) {
            Some(CidrMembership::Inside { host }) => {
                let words = Message::new(words::INSIDE)
                    .with("host", grouped(host, language))
                    .text(language);
                let said = paragraph(ui, &words, (12.5, 500.0), p.green);
                address(ui, &said, "cidr-membership");
            }
            Some(CidrMembership::Outside) => {
                let said = paragraph(ui, &tr(words::OUTSIDE), (12.5, 500.0), p.text_1);
                address(ui, &said, "cidr-membership");
            }
            Some(CidrMembership::Unreadable) => {
                problem(ui, &tr(words::MEMBER_UNREADABLE), "cidr-membership");
            }
            _ => {}
        }
    }

    fn facts_area(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::cidr as words;
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let Some(block) = self.block() else {
            hint(ui, &tr(words::EMPTY));
            return;
        };
        let family = family_id(block.family);
        let name = |id: &str| say(language, &format!("tools.cidr.rows.{id}.{family}"));
        let facts = fact_rows(block);
        let names: Vec<String> = facts.iter().map(|(id, _)| name(id)).collect();
        let usable = name("usable");
        let hosts = hosts_of(block, language);
        card(ui, "cidr-facts", |ui| {
            let mut rows: Vec<_> = facts
                .iter()
                .zip(&names)
                .map(|((_, value), name)| ResultRow::new(name, value))
                .collect();
            rows.push(ResultRow::new(&usable, &hosts).reading());
            result_rows(ui, &rows, language);
        });
        self.split_area(ui, language);
        self.masks_area(ui, language);
    }

    fn split_area(&mut self, ui: &mut Ui, language: Language) {
        let Some(answer) = self.answer.value() else {
            return;
        };
        let Some(split) = &answer.split else {
            return;
        };
        let Some(block) = &answer.block else {
            return;
        };
        section_title(
            ui,
            &i18n::translate(language, keys::tools::cidr::SPLIT_TITLE, &[]),
        );
        let mut chosen = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(6.0);
            for prefix in &answer.split_choices {
                let words = format!("/{prefix}");
                let id = format!("cidr-split-choice-{prefix}");
                if Button::new(&words, &id)
                    .armed(*prefix == split.prefix)
                    .show(ui)
                    .clicked()
                {
                    chosen = Some(*prefix);
                }
            }
        });
        subnets(ui, split, block.family, language);
        if chosen.is_some() {
            self.split = chosen;
        }
    }

    fn masks_area(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::cidr::masks as words;
        let tr = |key| i18n::translate(language, key, &[]);
        let Some(answer) = self.answer.value() else {
            return;
        };
        let Some(block) = &answer.block else {
            return;
        };
        section_title(ui, &tr(words::TITLE));
        hint(ui, &tr(words::HINT));
        let v4 = block.family == IpFamily::V4;
        let mut headers = vec![TextColumn::new(tr(words::PREFIX), 0.1)];
        if v4 {
            headers.push(TextColumn::new(tr(words::MASK), 0.23));
            headers.push(TextColumn::new(tr(words::WILDCARD), 0.23));
        }
        headers.push(TextColumn::new(tr(words::ADDRESSES), 0.22));
        let last = if v4 { words::HOSTS } else { words::NETWORKS64 };
        headers.push(TextColumn::new(tr(last), 0.22));
        let rows: Vec<Vec<String>> = answer
            .masks
            .iter()
            .map(|row| {
                let mut cells = vec![format!("/{}", row.prefix)];
                if v4 {
                    cells.push(row.mask.clone().unwrap_or_default());
                    cells.push(row.wildcard.clone().unwrap_or_default());
                }
                cells.push(mask_addresses(row, block.family, language));
                cells.push(if v4 {
                    grouped(&row.usable, language)
                } else {
                    row.networks64
                        .as_deref()
                        .map_or_else(|| "—".to_owned(), |it| grouped(it, language))
                });
                cells
            })
            .collect();
        let lit = answer
            .masks
            .iter()
            .position(|row| row.prefix == block.prefix);
        // Four numbers and two counts do not hold in a column: the table scrolls in its own.
        let picked = egui::ScrollArea::horizontal()
            .id_salt("cidr-masks-scroll")
            .show(ui, |ui| {
                ui.set_width(ui.available_width().max(MASKS_WIDTH));
                text_table(ui, "cidr-masks", &headers, &rows, lit)
            })
            .inner;
        if let Some(prefix) = picked
            .and_then(|at| answer.masks.get(at))
            .map(|it| it.prefix)
        {
            // On the address as it was typed, so going back finds the same block.
            self.network = format!("{}/{prefix}", block.typed);
            self.split = None;
        }
    }

    fn plan_area(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::cidr::plan as words;
        let tr = |key| i18n::translate(language, key, &[]);
        let name = tr(words::TITLE);
        section_title(ui, &name);
        hint(ui, &tr(words::HINT));
        let placeholder = tr(words::PLACEHOLDER);
        text_area(ui, &mut self.plan, (&name, "cidr-plan"), &placeholder, 5);
        if let Some(plan) = self.answer.value().and_then(|it| it.plan.as_ref()) {
            plan_table(ui, plan, language);
        } else if self.block().is_none() && !self.plan.trim().is_empty() {
            let said = hint(ui, &tr(words::WAITING));
            address(ui, &said, "cidr-plan-waiting");
        }
    }

    fn list_area(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::cidr::list as words;
        let tr = |key| i18n::translate(language, key, &[]);
        let name = tr(words::TITLE);
        section_title(ui, &name);
        hint(ui, &tr(words::HINT));
        let placeholder = tr(words::PLACEHOLDER);
        text_area(ui, &mut self.list, (&name, "cidr-list"), &placeholder, 5);
        if let Some(summary) = self.answer.value().and_then(|it| it.summary.as_ref()) {
            summary_of(ui, summary, language);
        }
    }

    fn plan_lines(&self, language: Language) -> Vec<String> {
        let placed: Vec<String> = self
            .answer
            .value()
            .and_then(|it| it.plan.as_ref())
            .into_iter()
            .flat_map(|plan| &plan.lines)
            .filter_map(|line| match line {
                PlanLine::Placed {
                    name,
                    cidr,
                    first,
                    last,
                    ..
                } => {
                    let name = if name.is_empty() { "—" } else { name };
                    Some(format!("{name} : {cidr} ({first} – {last})"))
                }
                _ => None,
            })
            .collect();
        if placed.is_empty() {
            return placed;
        }
        let title = i18n::translate(language, keys::tools::cidr::plan::TITLE, &[]);
        [vec![String::new(), title], placed].concat()
    }

    fn summary_lines(&self, language: Language) -> Vec<String> {
        let Some(summary) = self.answer.value().and_then(|it| it.summary.as_ref()) else {
            return Vec::new();
        };
        if summary.families.is_empty() {
            return Vec::new();
        }
        let supernet = i18n::translate(language, keys::tools::cidr::list::SUPERNET, &[]);
        let title = i18n::translate(language, keys::tools::cidr::list::TITLE, &[]);
        let mut lines = vec![String::new(), title];
        for family in &summary.families {
            lines.extend(family.blocks.iter().cloned());
            lines.push(format!("{supernet} : {}", family.supernet));
        }
        lines
    }
}

impl Tool for CidrTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        let request = CidrRequest {
            network: self.network.clone(),
            family: self.family,
            member: self.member.clone(),
            split: self.split,
            plan: self.plan.clone(),
            list: self.list.clone(),
        };
        self.answer
            .update(turn, Some(request), api::tools::describe_cidr);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::network_area, Self::facts_area));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let block = self.block()?;
        let family = family_id(block.family);
        let name = |id: &str| say(language, &format!("tools.cidr.rows.{id}.{family}"));
        let mut lines = vec![block.cidr.clone()];
        lines.extend(
            fact_rows(block)
                .iter()
                .map(|(id, value)| format!("{} : {value}", name(id))),
        );
        lines.push(format!(
            "{} : {}",
            name("usable"),
            hosts_of(block, language)
        ));
        lines.extend(self.plan_lines(language));
        lines.extend(self.summary_lines(language));
        Some(ToolResult {
            title: i18n::translate(language, keys::tools::cidr::NOTE_TITLE, &[]),
            language: NoteLanguage::Txt,
            content: lines.join("\n"),
            warning: None,
        })
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        self.family = None;
        "10.24.8.0/21".clone_into(&mut self.network);
        "10.24.12.40".clone_into(&mut self.member);
        self.split = None;
        SAMPLE_PLAN.clone_into(&mut self.plan);
        SAMPLE_LIST.clone_into(&mut self.list);
    }

    fn clear(&mut self) {
        self.network.clear();
        self.member.clear();
        self.split = None;
        self.plan.clear();
        self.list.clear();
    }
}
