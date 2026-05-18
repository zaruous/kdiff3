use std::path::Path;
use std::sync::Arc;

use anyhow::Result;

use crate::line_data::{parse_lines, LineData};

/// 단일 입력 파일의 데이터. KDiff3의 SourceData에 대응.
#[derive(Debug, Clone)]
pub struct SourceData {
    pub path: String,
    pub content: Arc<String>,
    pub lines: Vec<LineData>,
    pub encoding: &'static str,
}

impl SourceData {
    pub fn from_file(path: &Path) -> Result<Self> {
        let raw = std::fs::read(path)?;
        let (encoding, content) = detect_and_decode(&raw);
        let content = Arc::new(content);
        let lines = parse_lines(Arc::clone(&content));

        Ok(Self {
            path: path.to_string_lossy().to_string(),
            content,
            lines,
            encoding,
        })
    }

    pub fn from_str(label: &str, text: &str) -> Self {
        let content = Arc::new(text.to_string());
        let lines = parse_lines(Arc::clone(&content));
        Self {
            path: label.to_string(),
            content,
            lines,
            encoding: "UTF-8",
        }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

/// BOM 또는 UTF-8 감지 후 디코딩. 실패 시 Latin-1 폴백.
fn detect_and_decode(raw: &[u8]) -> (&'static str, String) {
    // UTF-8 BOM
    if raw.starts_with(b"\xEF\xBB\xBF") {
        return ("UTF-8", String::from_utf8_lossy(&raw[3..]).into_owned());
    }
    // UTF-16 LE BOM
    if raw.starts_with(b"\xFF\xFE") {
        let (decoded, _, _) = encoding_rs::UTF_16LE.decode(raw);
        return ("UTF-16LE", decoded.into_owned());
    }
    // UTF-16 BE BOM
    if raw.starts_with(b"\xFE\xFF") {
        let (decoded, _, _) = encoding_rs::UTF_16BE.decode(raw);
        return ("UTF-16BE", decoded.into_owned());
    }
    // UTF-8 시도
    if let Ok(s) = std::str::from_utf8(raw) {
        return ("UTF-8", s.to_owned());
    }
    // Latin-1 폴백
    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(raw);
    ("Windows-1252", decoded.into_owned())
}
