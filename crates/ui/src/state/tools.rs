//! Where the Tools area is: its home, one category of it, or one tool, kept across a trip to the
//! notes. The recent and the starred tools are remembered with the application.

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Value, json};

use crate::services::preferences::Preferences;
use crate::tools::catalogue::{self, Category, ToolEntry};
use engine::tools::identity::CsvSeparator;

const RECENTS: &str = "devnotes.recentTools";
const FAVOURITES: &str = "devnotes.favouriteTools";
const ZONES: &str = "devnotes.timeZones";
const CSV_SEPARATOR: &str = "devnotes.csvSeparator";

const DEFAULT_ZONES: [&str; 3] = ["UTC", "America/New_York", "Asia/Tokyo"];
/// Read back from a file nobody checks.
const KEPT_ZONES: usize = 64;

/// How many are shown, on the home and in the rail.
pub const SHOWN_RECENTS: usize = 5;
/// More than are shown: the rail leaves out the ones its section above lists, and still shows five.
const KEPT_RECENTS: usize = 16;
/// Read back from a file nobody checks: more than any catalogue holds.
const KEPT_FAVOURITES: usize = 64;

/// What the home is narrowed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    Category(Category),
    Favourites,
}

/// Which tool was opened, and when: nothing of what was typed in it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RecentTool {
    id: String,
    at: DateTime<Utc>,
}

pub struct ToolsState {
    open: Option<&'static ToolEntry>,
    filter: Option<Filter>,
    recents: Vec<RecentTool>,
    /// In the order they were starred; one the catalogue lost stays in the file.
    favourites: Vec<String>,
    /// Which of them the database knows is Rust's to say, not the file's.
    zones: Vec<String>,
    csv_separator: CsvSeparator,
}

impl Default for ToolsState {
    fn default() -> Self {
        Self {
            open: None,
            filter: None,
            recents: Vec::new(),
            favourites: Vec::new(),
            zones: DEFAULT_ZONES.map(str::to_owned).to_vec(),
            csv_separator: CsvSeparator::Comma,
        }
    }
}

impl ToolsState {
    pub fn read(preferences: &Preferences) -> Self {
        Self {
            recents: preferences.get(RECENTS).map_or_else(Vec::new, read_recents),
            favourites: preferences
                .get(FAVOURITES)
                .map_or_else(Vec::new, read_favourites),
            zones: preferences
                .get(ZONES)
                .map_or_else(|| DEFAULT_ZONES.map(str::to_owned).to_vec(), read_zones),
            csv_separator: preferences
                .get(CSV_SEPARATOR)
                .map_or(CsvSeparator::Comma, read_separator),
            ..Self::default()
        }
    }

    pub fn write(&self, preferences: &mut Preferences) {
        let recents: Vec<Value> = self
            .recents
            .iter()
            .map(|recent| {
                let at = recent.at.to_rfc3339_opts(SecondsFormat::Millis, true);
                json!({ "id": recent.id, "at": at })
            })
            .collect();
        preferences.set(RECENTS, &Value::from(recents).to_string());
        preferences.set(FAVOURITES, &json!(self.favourites).to_string());
        preferences.set(ZONES, &json!(self.zones).to_string());
        preferences.set(CSV_SEPARATOR, self.csv_separator.as_str());
    }

    pub fn zones(&self) -> &[String] {
        &self.zones
    }

    pub fn set_zones(&mut self, zones: Vec<String>) {
        self.zones = zones;
    }

    pub fn csv_separator(&self) -> CsvSeparator {
        self.csv_separator
    }

    pub fn set_csv_separator(&mut self, separator: CsvSeparator) {
        self.csv_separator = separator;
    }

    pub fn open_tool(&self) -> Option<&'static ToolEntry> {
        self.open
    }

    pub fn open(&mut self, entry: &'static ToolEntry, at: DateTime<Utc>) {
        self.open = Some(entry);
        self.recents.retain(|recent| recent.id != entry.id);
        let id = entry.id.to_owned();
        self.recents.insert(0, RecentTool { id, at });
        self.recents.truncate(KEPT_RECENTS);
    }

    /// The whole catalogue, one category of it, or the favourites.
    pub fn home(&mut self, filter: Option<Filter>) {
        self.open = None;
        self.filter = filter;
    }

    /// What the home shows, the favourites only while there is one.
    pub fn filter(&self) -> Option<Filter> {
        self.filter
            .filter(|filter| *filter != Filter::Favourites || !self.favourites().is_empty())
    }

    /// The recent tools the catalogue still has, latest first.
    pub fn recents(&self) -> impl Iterator<Item = (&'static ToolEntry, DateTime<Utc>)> + '_ {
        self.recents
            .iter()
            .filter_map(|recent| Some((catalogue::find(&recent.id)?, recent.at)))
    }

    /// The starred tools the catalogue still has, in the order they were starred.
    pub fn favourites(&self) -> Vec<&'static ToolEntry> {
        self.favourites
            .iter()
            .filter_map(|id| catalogue::find(id))
            .collect()
    }

    pub fn is_favourite(&self, id: &str) -> bool {
        self.favourites.iter().any(|starred| starred == id)
    }

    /// Starred goes last; unstarred, the home narrowed to the favourites widens once none is left.
    pub fn toggle_favourite(&mut self, id: &str) {
        if self.is_favourite(id) {
            self.favourites.retain(|starred| starred != id);
        } else {
            self.favourites.push(id.to_owned());
        }
        if self.favourites.is_empty() && self.filter == Some(Filter::Favourites) {
            self.filter = None;
        }
    }
}

fn read_recents(stored: &str) -> Vec<RecentTool> {
    let Ok(Value::Array(values)) = serde_json::from_str(stored) else {
        return Vec::new();
    };
    values
        .iter()
        .filter_map(|value| {
            let id = value.get("id")?.as_str()?.to_owned();
            let at = value.get("at")?.as_str()?.parse().ok()?;
            Some(RecentTool { id, at })
        })
        .take(KEPT_RECENTS)
        .collect()
}

/// Ids alone, each once: which of them the catalogue still has is for whoever lists them to say.
fn read_favourites(stored: &str) -> Vec<String> {
    read_names(stored, KEPT_FAVOURITES)
}

fn read_separator(stored: &str) -> CsvSeparator {
    CsvSeparator::ALL
        .into_iter()
        .find(|it| it.as_str() == stored)
        .unwrap_or_default()
}

fn read_zones(stored: &str) -> Vec<String> {
    read_names(stored, KEPT_ZONES)
}

fn read_names(stored: &str, kept: usize) -> Vec<String> {
    let Ok(Value::Array(values)) = serde_json::from_str(stored) else {
        return Vec::new();
    };
    let mut names: Vec<String> = Vec::new();
    for name in values.iter().filter_map(Value::as_str) {
        if !names.iter().any(|it| it == name) {
            names.push(name.to_owned());
        }
    }
    names.truncate(kept);
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str) -> &'static ToolEntry {
        catalogue::find(id).unwrap()
    }

    fn instant(text: &str) -> DateTime<Utc> {
        text.parse().unwrap()
    }

    #[test]
    fn a_tool_opened_again_comes_first_once() {
        let mut tools = ToolsState::default();
        tools.open(entry("case"), instant("2026-01-01T10:00:00Z"));
        tools.open(entry("hash"), instant("2026-01-01T10:01:00Z"));
        tools.open(entry("case"), instant("2026-01-01T10:02:00Z"));

        let ids: Vec<_> = tools.recents().map(|(tool, _)| tool.id).collect();

        assert_eq!(ids, ["case", "hash"]);
    }

    #[test]
    fn what_is_remembered_reads_back_and_a_lost_tool_is_not_listed() {
        let mut preferences = Preferences::default();
        let mut tools = ToolsState::default();
        tools.open(entry("cron"), instant("2026-01-01T10:00:00Z"));
        tools.toggle_favourite("jwt");
        tools.toggle_favourite("gone");
        tools.write(&mut preferences);

        let read = ToolsState::read(&preferences);

        assert_eq!(read.recents().next().unwrap().0.id, "cron");
        assert_eq!(
            read.favourites().iter().map(|t| t.id).collect::<Vec<_>>(),
            ["jwt"]
        );
        assert!(read.is_favourite("gone"));
    }

    #[test]
    fn a_file_that_does_not_read_is_nothing_remembered() {
        let mut preferences = Preferences::default();
        preferences.set(RECENTS, "not json");
        preferences.set(FAVOURITES, r#"{"id": 1}"#);

        let read = ToolsState::read(&preferences);

        assert_eq!(read.recents().count(), 0);
        assert!(read.favourites().is_empty());
    }

    #[test]
    fn the_last_favourite_gone_widens_the_home() {
        let mut tools = ToolsState::default();
        tools.toggle_favourite("jwt");
        tools.home(Some(Filter::Favourites));

        tools.toggle_favourite("jwt");

        assert_eq!(tools.filter(), None);
    }
}
