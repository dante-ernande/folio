use serde::{Deserialize, Serialize};

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn ago(t: u64) -> String {
    if t == 0 {
        return "never".into();
    }
    let d = now().saturating_sub(t);
    if d < 60 {
        "just now".into()
    } else if d < 3600 {
        format!("{} min ago", d / 60)
    } else if d < 86400 {
        format!("{} h ago", d / 3600)
    } else if d < 86400 * 30 {
        let n = d / 86400;
        format!("{} day{} ago", n, if n == 1 { "" } else { "s" })
    } else {
        format!("{} months ago", d / (86400 * 30))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Bookmark {
    pub chapter: usize,
    pub frac: f32,
    pub label: String,
    pub snippet: String,
    pub created: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Source {
    Builtin(usize),
    File(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub source: Source,
    pub title: String,
    pub author: String,
    pub chapters: usize,
    pub chapter: usize,
    pub frac: f32,
    pub added: u64,
    pub last_opened: u64,
    pub favorite: bool,
    pub finished: bool,
    pub bookmarks: Vec<Bookmark>,
}

impl Entry {
    pub fn progress(&self) -> f32 {
        if self.finished {
            1.0
        } else if self.chapters == 0 {
            0.0
        } else {
            ((self.chapter as f32 + self.frac) / self.chapters as f32).clamp(0.0, 1.0)
        }
    }
    pub fn started(&self) -> bool {
        self.last_opened > 0 && !self.finished
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum Sort {
    Recent,
    Title,
    Author,
    Progress,
    Added,
}

impl Sort {
    pub const ALL: [Sort; 5] = [Sort::Recent, Sort::Title, Sort::Author, Sort::Progress, Sort::Added];
    pub fn label(self) -> &'static str {
        match self {
            Sort::Recent => "Recently read",
            Sort::Title => "Title",
            Sort::Author => "Author",
            Sort::Progress => "Progress",
            Sort::Added => "Date added",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub theme: usize,
    pub font_size: f32,
    pub line_spacing: f32,
    pub column_width: f32,
    pub serif: bool,
    pub justify: bool,
    pub indent: bool,
    pub grid: bool,
    pub sort: Sort,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: 0,
            font_size: 19.0,
            line_spacing: 1.5,
            column_width: 720.0,
            serif: true,
            justify: false,
            indent: false,
            grid: true,
            sort: Sort::Recent,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Saved {
    pub initialized: bool,
    pub entries: Vec<Entry>,
    pub settings: Settings,
}
