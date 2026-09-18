//! The component catalog (`CATALOG.toml`): every framework item, its group, priority and status.

use toml::de::{DeTable, DeValue};

/// The catalog file, compiled in.
pub const SOURCE: &str = include_str!("../CATALOG.toml");

/// Groups in menu order.
pub const GROUPS: [&str; 8] =
    ["foundations", "controls", "display", "structure", "overlays", "forms", "content", "examples"];

/// One catalog entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub group: String,
    pub kind: String,
    pub priority: String,
    pub done: bool,
    pub page: Option<String>,
    pub types: Vec<String>,
    /// Design notes in English and Turkish, shown on the planned page.
    pub notes: Option<[String; 2]>,
}

/// The parsed catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    pub items: Vec<Item>,
}

impl Catalog {
    /// Parses the compiled-in catalog.
    ///
    /// # Panics
    ///
    /// Panics when the file is malformed; the catalog tests keep it valid.
    #[must_use]
    pub fn load() -> Self {
        Self::parse(SOURCE).unwrap_or_else(|message| panic!("CATALOG.toml: {message}"))
    }

    /// Parses catalog text.
    pub fn parse(text: &str) -> Result<Self, String> {
        let root = DeTable::parse(text).map_err(|e| e.message().to_owned())?.into_inner();
        let items = root.get("item").and_then(|v| v.get_ref().as_array()).ok_or("missing [[item]] entries")?;
        let mut parsed = Vec::new();
        for entry in items.iter() {
            let table = entry.get_ref().as_table().ok_or("an [[item]] is not a table")?;
            let text = |key: &str| -> Result<String, String> {
                table
                    .get(key)
                    .and_then(|v| v.get_ref().as_str())
                    .map(str::to_owned)
                    .ok_or_else(|| format!("an item is missing `{key}`"))
            };
            let optional = |key: &str| table.get(key).and_then(|v| v.get_ref().as_str()).map(str::to_owned);
            let status = text("status")?;
            let types = match table.get("types").map(toml::Spanned::get_ref) {
                Some(DeValue::Array(values)) => {
                    values.iter().filter_map(|v| v.get_ref().as_str().map(str::to_owned)).collect()
                }
                _ => Vec::new(),
            };
            parsed.push(Item {
                id: text("id")?,
                name: text("name")?,
                group: text("group")?,
                kind: text("kind")?,
                priority: text("priority")?,
                done: match status.as_str() {
                    "done" => true,
                    "planned" => false,
                    other => return Err(format!("unknown status `{other}`")),
                },
                page: optional("page"),
                types,
                notes: match (optional("notes"), optional("notes-tr")) {
                    (Some(en), Some(tr)) => Some([en, tr]),
                    (None, None) => None,
                    _ => return Err("an item has `notes` without `notes-tr` or the other way round".to_owned()),
                },
            });
        }
        Ok(Self { items: parsed })
    }

    /// The item with `id`.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|item| item.id == id)
    }
}
