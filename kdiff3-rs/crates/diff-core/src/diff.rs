use similar::{ChangeTag, TextDiff};

use crate::line_data::LineData;

/// 입력 소스 선택자. A=파일1(base), B=파일2, C=파일3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SrcSelector {
    None,
    A,
    B,
    C,
}

impl SrcSelector {
    pub fn next(self) -> Self {
        match self {
            SrcSelector::None => SrcSelector::A,
            SrcSelector::A => SrcSelector::B,
            SrcSelector::B => SrcSelector::C,
            SrcSelector::C => SrcSelector::None,
        }
    }
}

/// 두 구간 사이의 단일 diff 단위.
/// nof_equals: 동일한 라인 수
/// diff1, diff2: 각각 다른 라인 수
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    pub nof_equals: usize,
    pub diff1: usize,
    pub diff2: usize,
}

impl Diff {
    pub fn new(nof_equals: usize, diff1: usize, diff2: usize) -> Self {
        Self { nof_equals, diff1, diff2 }
    }

    pub fn is_empty(&self) -> bool {
        self.nof_equals == 0 && self.diff1 == 0 && self.diff2 == 0
    }

    /// 동일 구간이 4줄 미만이면 diff로 흡수 (노이즈 제거)
    pub fn refine(&mut self) {
        if self.nof_equals < 4 && (self.diff1 > 0 || self.diff2 > 0) {
            self.diff1 += self.nof_equals;
            self.diff2 += self.nof_equals;
            self.nof_equals = 0;
        }
    }
}

/// Diff 목록. 두 파일의 전체 diff 결과를 담음.
#[derive(Debug, Clone, Default)]
pub struct DiffList(pub Vec<Diff>);

impl DiffList {
    /// similar 크레이트로 라인 단위 diff 계산
    pub fn from_lines(lines_a: &[LineData], lines_b: &[LineData]) -> Self {
        let text_a: Vec<&str> = lines_a.iter().map(|l| l.get_line()).collect();
        let text_b: Vec<&str> = lines_b.iter().map(|l| l.get_line()).collect();

        let diff = TextDiff::from_slices(&text_a, &text_b);
        let mut result = Vec::new();

        let mut nof_eq = 0usize;
        let mut del = 0usize;
        let mut ins = 0usize;

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal => {
                    if del > 0 || ins > 0 {
                        result.push(Diff::new(0, del, ins));
                        del = 0;
                        ins = 0;
                    }
                    nof_eq += 1;
                }
                ChangeTag::Delete => {
                    if nof_eq > 0 {
                        result.push(Diff::new(nof_eq, 0, 0));
                        nof_eq = 0;
                    }
                    del += 1;
                }
                ChangeTag::Insert => {
                    if nof_eq > 0 {
                        result.push(Diff::new(nof_eq, 0, 0));
                        nof_eq = 0;
                    }
                    ins += 1;
                }
            }
        }

        if nof_eq > 0 || del > 0 || ins > 0 {
            result.push(Diff::new(nof_eq, del, ins));
        }

        DiffList(result)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diff> {
        self.0.iter()
    }
}

/// 라인 범위 (시작 라인 인덱스, 길이)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffRange {
    pub start: usize,
    pub len: usize,
}

impl DiffRange {
    pub fn new(start: usize, len: usize) -> Self {
        Self { start, len }
    }

    pub fn end(&self) -> usize {
        self.start + self.len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::line_data::parse_lines;
    use std::sync::Arc;

    #[test]
    fn test_diff_identical_files() {
        let content = Arc::new("line1\nline2\nline3".to_string());
        let lines = parse_lines(Arc::clone(&content));
        let diff = DiffList::from_lines(&lines, &lines);
        // 동일 파일은 diff1/diff2 가 모두 0
        assert!(diff.0.iter().all(|d| d.diff1 == 0 && d.diff2 == 0));
    }

    #[test]
    fn test_diff_changed_line() {
        let a = Arc::new("line1\nline2\nline3".to_string());
        let b = Arc::new("line1\nchanged\nline3".to_string());
        let la = parse_lines(a);
        let lb = parse_lines(b);
        let diff = DiffList::from_lines(&la, &lb);
        let has_diff = diff.0.iter().any(|d| d.diff1 > 0 || d.diff2 > 0);
        assert!(has_diff);
    }
}
