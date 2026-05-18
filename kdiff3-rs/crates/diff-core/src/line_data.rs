use std::sync::Arc;

/// 파일의 단일 라인. Arc<String> 버퍼를 공유해 복사 없이 슬라이싱.
/// KDiff3 C++의 LineData에 대응.
#[derive(Clone, Debug)]
pub struct LineData {
    buffer: Arc<String>,
    offset: usize,
    size: usize,
    first_non_white: usize,
    is_pure_comment: bool,
    is_skipable: bool,
}

impl LineData {
    pub fn new(
        buffer: Arc<String>,
        offset: usize,
        size: usize,
        first_non_white: usize,
        is_pure_comment: bool,
        is_skipable: bool,
    ) -> Self {
        Self { buffer, offset, size, first_non_white, is_pure_comment, is_skipable }
    }

    pub fn get_line(&self) -> &str {
        &self.buffer[self.offset..self.offset + self.size]
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn first_non_white(&self) -> usize {
        self.first_non_white
    }

    pub fn is_white_line(&self) -> bool {
        self.first_non_white == 0
    }

    pub fn is_pure_comment(&self) -> bool {
        self.is_pure_comment
    }

    pub fn is_skipable(&self) -> bool {
        self.is_skipable
    }

    /// 탭을 고려한 시각적 너비 계산
    pub fn visual_width(&self, tab_size: usize) -> usize {
        let mut width = 0;
        for ch in self.get_line().chars() {
            if ch == '\t' {
                width = ((width / tab_size) + 1) * tab_size;
            } else {
                width += 1;
            }
        }
        width
    }
}

/// 파일 전체를 LineData 벡터로 파싱
pub fn parse_lines(content: Arc<String>) -> Vec<LineData> {
    let mut lines = Vec::new();
    let mut offset = 0;

    for line in content.split('\n') {
        let size = line.len();
        let first_non_white = line.chars().take_while(|c| c.is_whitespace()).count();
        lines.push(LineData::new(
            Arc::clone(&content),
            offset,
            size,
            first_non_white,
            false,
            false,
        ));
        offset += size + 1; // +1 for '\n'
    }

    lines
}
