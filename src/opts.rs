use serde::{Deserialize, Serialize};

// Supported format enum
#[derive(Debug, Serialize, Deserialize)]
pub enum ThumbnailFormat {
    #[serde(rename = "png")]
    Png,
    #[serde(rename = "jpeg")]
    Jpeg,
    #[serde(rename = "svg")]
    Svg,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThumbnailOptions {
    pub dpi: u32,
    pub format: ThumbnailFormat,
}
