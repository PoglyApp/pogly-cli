use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhoAmI {
    pub token_id: u64,
    pub label: String,
    pub read_only: bool,
    pub identity: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Element {
    pub id: u32,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub x: i64,
    #[serde(default)]
    pub y: i64,
    #[serde(default)]
    pub layout_id: u32,
    #[serde(default)]
    pub locked: bool,
    pub text: Option<TextGroup>,
    pub image: Option<ImageGroup>,
    pub widget: Option<WidgetGroup>,
    pub media: Option<MediaGroup>,
}

impl Element {
    pub fn summary(&self) -> String {
        if let Some(t) = &self.text {
            t.text.clone()
        } else if let Some(i) = &self.image {
            i.url
                .clone()
                .unwrap_or_else(|| format!("data #{}", i.element_data_id.unwrap_or(0)))
        } else if let Some(w) = &self.widget {
            format!("data #{}", w.element_data_id)
        } else if let Some(m) = &self.media {
            m.source.clone()
        } else {
            String::new()
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextGroup {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub size: i64,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub font: String,
    #[serde(default)]
    pub css: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageGroup {
    #[serde(default)]
    pub width: i64,
    #[serde(default)]
    pub height: i64,
    pub element_data_id: Option<u32>,
    pub url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetGroup {
    #[serde(default)]
    pub element_data_id: u32,
    #[serde(default)]
    pub width: i64,
    #[serde(default)]
    pub height: i64,
    // Only present when the list was fetched with ?raw=true.
    pub raw_data: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaGroup {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub volume: i64,
    #[serde(default)]
    pub width: i64,
    #[serde(default)]
    pub height: i64,
    #[serde(default)]
    pub playing: bool,
    #[serde(default)]
    pub autoplay: bool,
    #[serde(default, rename = "loop")]
    pub loop_: bool,
    #[serde(default)]
    pub timestamp: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementData {
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub width: i64,
    #[serde(default)]
    pub height: i64,
    #[serde(default)]
    pub folder_id: u32,
    #[serde(default)]
    pub created_by: String,
}

#[derive(Deserialize)]
pub struct Layout {
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub active: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub created_by: String,
}
