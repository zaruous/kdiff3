use std::ops::Range;

use crate::diff::DiffList;
use crate::line_data::LineData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedChoice {
    A,
    B,
    C,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeDetails {
    NoChange,
    BChanged,
    CChanged,
    BCChanged,          // 충돌: B와 C가 각각 다르게 변경
    BCChangedAndEqual,  // B와 C가 같은 내용으로 변경 → 자동 해결
    BDeleted,
    CDeleted,
    BCDeleted,
    BAdded,
    CAdded,
    BCAdded,            // 충돌: B와 C가 각각 다른 내용 추가
    BCAddedAndEqual,    // B와 C가 같은 내용 추가 → 자동 해결
}

impl MergeDetails {
    pub fn is_conflict(&self) -> bool {
        matches!(self, Self::BCChanged | Self::BCAdded)
    }

    /// 사용자 개입 없이 해결 가능한 케이스의 자동 선택
    pub fn auto_choice(&self) -> Option<ResolvedChoice> {
        match self {
            Self::NoChange | Self::BCChangedAndEqual | Self::BCAddedAndEqual | Self::BCDeleted => {
                Some(ResolvedChoice::A)
            }
            Self::BChanged | Self::BAdded | Self::CDeleted => Some(ResolvedChoice::B),
            Self::CChanged | Self::CAdded | Self::BDeleted => Some(ResolvedChoice::C),
            _ => None,
        }
    }
}

/// 병합 뷰의 단일 블록. 동일 라인 그룹 또는 변경/충돌 블록.
#[derive(Debug, Clone)]
pub struct MergeBlock {
    pub a_range: Range<usize>,
    pub b_range: Range<usize>,
    pub c_range: Range<usize>,
    pub details: MergeDetails,
    pub resolved: Option<ResolvedChoice>,
}

impl MergeBlock {
    pub fn is_unresolved_conflict(&self) -> bool {
        self.details.is_conflict() && self.resolved.is_none()
    }

    /// 이 블록의 출력 선택. 미해결 충돌은 None.
    pub fn output_choice(&self) -> Option<ResolvedChoice> {
        self.resolved.or_else(|| self.details.auto_choice())
    }
}

// ── 내부 헬퍼 ────────────────────────────────────────────────────────────────

struct Span {
    a_start: usize,
    a_len: usize,
    o_start: usize,
    o_len: usize,
    equal: bool,
}

fn to_spans(diff: &DiffList) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut a = 0usize;
    let mut o = 0usize;
    for d in diff.iter() {
        if d.nof_equals > 0 {
            spans.push(Span {
                a_start: a,
                a_len: d.nof_equals,
                o_start: o,
                o_len: d.nof_equals,
                equal: true,
            });
            a += d.nof_equals;
            o += d.nof_equals;
        }
        if d.diff1 > 0 || d.diff2 > 0 {
            spans.push(Span {
                a_start: a,
                a_len: d.diff1,
                o_start: o,
                o_len: d.diff2,
                equal: false,
            });
            a += d.diff1;
            o += d.diff2;
        }
    }
    spans
}

/// A 위치에 대응하는 "other"(B 또는 C) 파일 위치를 반환.
/// equal 구간은 1:1, changed 구간은 블록 시작으로 매핑.
fn a_to_other(a_pos: usize, spans: &[Span]) -> usize {
    let mut a = 0usize;
    let mut o = 0usize;
    for s in spans {
        let a_next = a + s.a_len;
        if a_pos < a_next {
            return if s.equal { o + (a_pos - a) } else { o };
        }
        a = a_next;
        o += s.o_len;
    }
    o
}

fn ranges_content_equal(
    br: &Range<usize>,
    lines_b: &[LineData],
    cr: &Range<usize>,
    lines_c: &[LineData],
) -> bool {
    if br.len() != cr.len() {
        return false;
    }
    br.clone().zip(cr.clone()).all(|(bi, ci)| {
        lines_b.get(bi).map(|l| l.get_line()) == lines_c.get(ci).map(|l| l.get_line())
    })
}

// ── Merger ───────────────────────────────────────────────────────────────────

pub struct Merger {
    pub blocks: Vec<MergeBlock>,
}

impl Merger {
    /// 2-way 병합 (파일 A, B)
    pub fn from_twoway(diff_ab: &DiffList) -> Self {
        let mut blocks = Vec::new();
        let mut a = 0usize;
        let mut b = 0usize;
        for d in diff_ab.iter() {
            if d.nof_equals > 0 {
                blocks.push(MergeBlock {
                    a_range: a..a + d.nof_equals,
                    b_range: b..b + d.nof_equals,
                    c_range: 0..0,
                    details: MergeDetails::NoChange,
                    resolved: None,
                });
                a += d.nof_equals;
                b += d.nof_equals;
            }
            if d.diff1 > 0 || d.diff2 > 0 {
                let details = match (d.diff1, d.diff2) {
                    (0, _) => MergeDetails::BAdded,
                    (_, 0) => MergeDetails::BDeleted,
                    _ => MergeDetails::BChanged,
                };
                blocks.push(MergeBlock {
                    a_range: a..a + d.diff1,
                    b_range: b..b + d.diff2,
                    c_range: 0..0,
                    details,
                    resolved: None,
                });
                a += d.diff1;
                b += d.diff2;
            }
        }
        Self { blocks }
    }

    /// 3-way 병합 (파일 A 기준으로 B, C 비교)
    pub fn from_threeway(
        diff_ab: &DiffList,
        diff_ac: &DiffList,
        lines_b: &[LineData],
        lines_c: &[LineData],
    ) -> Self {
        let spans_ab = to_spans(diff_ab);
        let spans_ac = to_spans(diff_ac);
        let blocks = build_threeway_blocks(&spans_ab, &spans_ac, lines_b, lines_c);
        Self { blocks }
    }

    pub fn conflict_count(&self) -> usize {
        self.blocks.iter().filter(|b| b.details.is_conflict()).count()
    }

    pub fn unresolved_count(&self) -> usize {
        self.blocks.iter().filter(|b| b.is_unresolved_conflict()).count()
    }

    pub fn resolve(&mut self, idx: usize, choice: ResolvedChoice) {
        if let Some(b) = self.blocks.get_mut(idx) {
            b.resolved = Some(choice);
        }
    }

    /// `from` 이후의 첫 번째 미해결 충돌 블록 인덱스
    pub fn next_conflict_after(&self, from: usize) -> Option<usize> {
        (from..self.blocks.len()).find(|&i| self.blocks[i].is_unresolved_conflict())
    }

    /// `from` 이전의 마지막 미해결 충돌 블록 인덱스
    pub fn prev_conflict_before(&self, from: usize) -> Option<usize> {
        (0..from).rev().find(|&i| self.blocks[i].is_unresolved_conflict())
    }

    /// 병합 결과 라인 목록. 미해결 충돌은 `<<<<<<<` 마커로 출력.
    pub fn output(
        &self,
        lines_a: &[LineData],
        lines_b: &[LineData],
        lines_c: &[LineData],
    ) -> Vec<String> {
        let mut out = Vec::new();
        for block in &self.blocks {
            match block.output_choice() {
                Some(ResolvedChoice::A) => {
                    for i in block.a_range.clone() {
                        if let Some(l) = lines_a.get(i) {
                            out.push(l.get_line().to_string());
                        }
                    }
                }
                Some(ResolvedChoice::B) => {
                    for i in block.b_range.clone() {
                        if let Some(l) = lines_b.get(i) {
                            out.push(l.get_line().to_string());
                        }
                    }
                }
                Some(ResolvedChoice::C) => {
                    for i in block.c_range.clone() {
                        if let Some(l) = lines_c.get(i) {
                            out.push(l.get_line().to_string());
                        }
                    }
                }
                None => {
                    out.push("<<<<<<< A (base)".to_string());
                    for i in block.a_range.clone() {
                        if let Some(l) = lines_a.get(i) {
                            out.push(l.get_line().to_string());
                        }
                    }
                    out.push("||||||| B".to_string());
                    for i in block.b_range.clone() {
                        if let Some(l) = lines_b.get(i) {
                            out.push(l.get_line().to_string());
                        }
                    }
                    if !block.c_range.is_empty() {
                        out.push("======= C".to_string());
                        for i in block.c_range.clone() {
                            if let Some(l) = lines_c.get(i) {
                                out.push(l.get_line().to_string());
                            }
                        }
                    }
                    out.push(">>>>>>>".to_string());
                }
            }
        }
        out
    }
}

// ── 3-way 블록 계산 ───────────────────────────────────────────────────────────

fn build_threeway_blocks(
    spans_ab: &[Span],
    spans_ac: &[Span],
    lines_b: &[LineData],
    lines_c: &[LineData],
) -> Vec<MergeBlock> {
    let mut blocks: Vec<MergeBlock> = Vec::new();
    let mut ci = 0usize; // spans_ac 커서

    for sb in spans_ab {
        let a_end = sb.a_start + sb.a_len;

        // sb 시작 이전에 끝나는 C 스팬들 처리 (B가 equal로 통과한 구간에서 C가 먼저 변경)
        while ci < spans_ac.len() {
            let sc = &spans_ac[ci];
            if sc.a_start + sc.a_len > sb.a_start {
                break;
            }
            if !sc.equal {
                let bp = a_to_other(sc.a_start, spans_ab);
                let details = classify_c_only(sc.a_len, sc.o_len);
                blocks.push(MergeBlock {
                    a_range: sc.a_start..sc.a_start + sc.a_len,
                    b_range: bp..bp + sc.a_len,
                    c_range: sc.o_start..sc.o_start + sc.o_len,
                    details,
                    resolved: None,
                });
            }
            ci += 1;
        }

        if sb.equal {
            // B는 이 A 구간을 그대로 유지. C가 이 구간을 어떻게 바꿨는지 확인.
            process_equal_span(sb, a_end, spans_ac, &mut ci, &mut blocks);
        } else {
            // B가 이 A 구간을 변경. C 스팬들과 겹치는지 확인.
            process_changed_span(sb, a_end, spans_ac, &mut ci, lines_b, lines_c, &mut blocks);
        }
    }

    // 남은 C 스팬 (B 스팬이 끝난 후)
    while ci < spans_ac.len() {
        let sc = &spans_ac[ci];
        if !sc.equal {
            let bp = a_to_other(sc.a_start, spans_ab);
            let details = classify_c_only(sc.a_len, sc.o_len);
            blocks.push(MergeBlock {
                a_range: sc.a_start..sc.a_start + sc.a_len,
                b_range: bp..bp + sc.a_len,
                c_range: sc.o_start..sc.o_start + sc.o_len,
                details,
                resolved: None,
            });
        }
        ci += 1;
    }

    blocks
}

/// B가 equal인 구간 처리: C가 변경한 sub-range들을 찾아 분리
#[allow(unused_assignments)]
fn process_equal_span(
    sb: &Span,
    a_end: usize,
    spans_ac: &[Span],
    ci: &mut usize,
    blocks: &mut Vec<MergeBlock>,
) {
    let b_base = sb.o_start;
    let mut a_cur = sb.a_start;

    loop {
        // a_cur 위치의 C 삽입(a_len == 0) 먼저 처리
        while *ci < spans_ac.len()
            && spans_ac[*ci].a_start == a_cur
            && spans_ac[*ci].a_len == 0
        {
            let sc = &spans_ac[*ci];
            let bp = b_base + (a_cur - sb.a_start);
            blocks.push(MergeBlock {
                a_range: a_cur..a_cur,
                b_range: bp..bp,
                c_range: sc.o_start..sc.o_start + sc.o_len,
                details: MergeDetails::CAdded,
                resolved: None,
            });
            *ci += 1;
        }

        if a_cur >= a_end {
            break;
        }

        // [a_cur, a_end) 범위에서 다음 C 변경 구간 찾기
        let next_idx = spans_ac[*ci..]
            .iter()
            .position(|sc| !sc.equal && sc.a_start >= a_cur && sc.a_start < a_end)
            .map(|pos| *ci + pos);

        if let Some(sc_idx) = next_idx {
            let sc_a_start = spans_ac[sc_idx].a_start;

            // sc 이전까지 NoChange 블록
            if a_cur < sc_a_start {
                let len = sc_a_start - a_cur;
                let bp = b_base + (a_cur - sb.a_start);
                let cp = a_to_other(a_cur, spans_ac);
                blocks.push(MergeBlock {
                    a_range: a_cur..a_cur + len,
                    b_range: bp..bp + len,
                    c_range: cp..cp + len,
                    details: MergeDetails::NoChange,
                    resolved: None,
                });
                a_cur = sc_a_start;
            }

            // C 변경 블록 (a_end를 넘어가지 않도록 클램핑)
            let sc = &spans_ac[sc_idx];
            let sc_a_len = sc.a_len.min(a_end - sc.a_start);
            let bp = b_base + (sc.a_start - sb.a_start);
            let details = classify_c_only(sc.a_len, sc.o_len);
            blocks.push(MergeBlock {
                a_range: sc.a_start..sc.a_start + sc_a_len,
                b_range: bp..bp + sc_a_len,
                c_range: sc.o_start..sc.o_start + sc.o_len,
                details,
                resolved: None,
            });
            a_cur = sc.a_start + sc_a_len;

            // ci를 sc 이후로 전진
            while *ci < spans_ac.len()
                && spans_ac[*ci].a_start + spans_ac[*ci].a_len <= a_cur
            {
                *ci += 1;
            }
        } else {
            // 남은 범위는 전부 NoChange
            let len = a_end - a_cur;
            let bp = b_base + (a_cur - sb.a_start);
            let cp = a_to_other(a_cur, spans_ac);
            blocks.push(MergeBlock {
                a_range: a_cur..a_end,
                b_range: bp..bp + len,
                c_range: cp..cp + len,
                details: MergeDetails::NoChange,
                resolved: None,
            });
            break;
        }
    }
}

/// B가 changed인 구간 처리: C가 같은 A 범위를 변경했는지 확인
fn process_changed_span(
    sb: &Span,
    a_end: usize,
    spans_ac: &[Span],
    ci: &mut usize,
    lines_b: &[LineData],
    lines_c: &[LineData],
    blocks: &mut Vec<MergeBlock>,
) {
    // [sb.a_start, a_end) 범위와 겹치는 C 변경 스팬 수집
    let mut c_changed = false;
    let mut c_o_start = 0usize;
    let mut c_o_end = 0usize;

    let mut k = *ci;
    while k < spans_ac.len() && spans_ac[k].a_start < a_end {
        let sc = &spans_ac[k];
        if !sc.equal {
            if !c_changed {
                c_o_start = sc.o_start;
            }
            c_o_end = sc.o_start + sc.o_len;
            c_changed = true;
        }
        k += 1;
    }

    // 이 B 스팬에 완전히 포함된 C 스팬들은 ci 전진
    while *ci < spans_ac.len()
        && spans_ac[*ci].a_start + spans_ac[*ci].a_len <= a_end
    {
        *ci += 1;
    }

    let b_range = sb.o_start..sb.o_start + sb.o_len;

    let (details, c_range) = if !c_changed {
        let cp = a_to_other(sb.a_start, spans_ac);
        let details = match (sb.a_len, sb.o_len) {
            (0, _) => MergeDetails::BAdded,
            (_, 0) => MergeDetails::BDeleted,
            _ => MergeDetails::BChanged,
        };
        (details, cp..cp + sb.a_len)
    } else {
        let c_range = c_o_start..c_o_end;
        let details = if ranges_content_equal(&b_range, lines_b, &c_range, lines_c) {
            MergeDetails::BCChangedAndEqual
        } else {
            MergeDetails::BCChanged
        };
        (details, c_range)
    };

    blocks.push(MergeBlock {
        a_range: sb.a_start..a_end,
        b_range,
        c_range,
        details,
        resolved: None,
    });
}

fn classify_c_only(a_len: usize, c_len: usize) -> MergeDetails {
    match (a_len, c_len) {
        (0, _) => MergeDetails::CAdded,
        (_, 0) => MergeDetails::CDeleted,
        _ => MergeDetails::CChanged,
    }
}

// ── lib.rs 재내보내기용 타입 추가 ─────────────────────────────────────────────

pub use ResolvedChoice as Choice;
