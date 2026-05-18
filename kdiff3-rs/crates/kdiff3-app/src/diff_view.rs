use diff_core::{DiffList, SourceData};
use egui::{Color32, FontId, RichText, ScrollArea, Ui};
use egui::text::LayoutJob;

const COLOR_ONLY_A: Color32 = Color32::from_rgb(255, 200, 200);
const COLOR_ONLY_B: Color32 = Color32::from_rgb(200, 225, 255);
const COLOR_ONLY_C: Color32 = Color32::from_rgb(200, 255, 200);
const COLOR_CHANGED: Color32 = Color32::from_rgb(255, 240, 180);
// 단어 단위 변경 강조색 (라인 배경보다 진함)
const COLOR_WORD_A: Color32 = Color32::from_rgb(220, 100, 100);
const COLOR_WORD_B: Color32 = Color32::from_rgb(100, 140, 220);

#[derive(Clone, Copy, PartialEq, Eq)]
enum LineKind {
    Equal,
    OnlyA,
    OnlyB,
    #[allow(dead_code)]
    OnlyC,
    ChangedAB,
}

#[derive(Clone)]
struct DiffLine {
    num_a: Option<usize>,
    num_b: Option<usize>,
    num_c: Option<usize>,
    text_a: String,
    text_b: String,
    text_c: String,
    kind: LineKind,
}

#[derive(PartialEq, Eq)]
enum ViewMode { TwoWay, ThreeWay }

/// 비교 텍스트 뷰. KDiff3의 DiffTextWindow에 대응.
pub struct DiffView {
    lines: Vec<DiffLine>,
    show_line_numbers: bool,
    mode: ViewMode,
    stats: DiffStats,
}

#[derive(Default)]
struct DiffStats {
    additions: usize,
    deletions: usize,
    changes: usize,
}

impl DiffView {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            show_line_numbers: true,
            mode: ViewMode::TwoWay,
            stats: DiffStats::default(),
        }
    }

    pub fn set_sources(
        &mut self,
        a: Option<SourceData>,
        b: Option<SourceData>,
        c: Option<SourceData>,
        diff_ab: DiffList,
        _diff_ac: Option<DiffList>,
    ) {
        self.mode = if c.is_some() { ViewMode::ThreeWay } else { ViewMode::TwoWay };
        self.lines = build_diff_lines(a.as_ref(), b.as_ref(), c.as_ref(), &diff_ab);
        self.stats = compute_stats(&diff_ab);
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let font = FontId::monospace(13.0);
        let is_3way = self.mode == ViewMode::ThreeWay;

        // ── 툴바 ──────────────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_line_numbers, "줄 번호");
            ui.separator();
            ui.label(
                RichText::new(format!(
                    "+{} 추가  -{} 삭제  ~{} 변경",
                    self.stats.additions, self.stats.deletions, self.stats.changes
                ))
                .color(Color32::GRAY),
            );
        });

        ui.separator();

        // ── 헤더 행 ───────────────────────────────────────────────────────────
        let col_count = if is_3way { 3 } else { 2 };
        ui.columns(col_count, |cols| {
            cols[0].label(RichText::new("파일 A  (base)").strong());
            cols[1].label(RichText::new("파일 B").strong());
            if is_3way {
                cols[2].label(RichText::new("파일 C").strong());
            }
        });

        ui.separator();

        // ── 본문: 하나의 ScrollArea 안에서 컬럼별 렌더링 ─────────────────────
        // 단일 ScrollArea → 양 컬럼이 자동으로 동기화 스크롤
        ScrollArea::vertical().id_source("diff_scroll").show(ui, |ui| {
            for line in &self.lines {
                let (bg_a, bg_b, bg_c) = match line.kind {
                    LineKind::Equal => (Color32::TRANSPARENT, Color32::TRANSPARENT, Color32::TRANSPARENT),
                    LineKind::OnlyA => (COLOR_ONLY_A, Color32::TRANSPARENT, Color32::TRANSPARENT),
                    LineKind::OnlyB => (Color32::TRANSPARENT, COLOR_ONLY_B, Color32::TRANSPARENT),
                    LineKind::OnlyC => (Color32::TRANSPARENT, Color32::TRANSPARENT, COLOR_ONLY_C),
                    LineKind::ChangedAB => (COLOR_CHANGED, COLOR_CHANGED, Color32::TRANSPARENT),
                };

                ui.columns(col_count, |cols| {
                    if line.kind == LineKind::ChangedAB {
                        // 단어 단위 하이라이팅
                        let job_a = word_diff_job(
                            line.num_a, self.show_line_numbers, &font,
                            &line.text_a, &line.text_b, true,
                            COLOR_CHANGED, COLOR_WORD_A,
                        );
                        cols[0].add(egui::Label::new(job_a));
                        let job_b = word_diff_job(
                            line.num_b, self.show_line_numbers, &font,
                            &line.text_a, &line.text_b, false,
                            COLOR_CHANGED, COLOR_WORD_B,
                        );
                        cols[1].add(egui::Label::new(job_b));
                    } else {
                        let text_a = if self.show_line_numbers {
                            match line.num_a {
                                Some(n) => format!("{:>4} │ {}", n, line.text_a),
                                None => "     │".to_string(),
                            }
                        } else {
                            line.text_a.clone()
                        };
                        cols[0].add(egui::Label::new(
                            RichText::new(text_a).font(font.clone()).background_color(bg_a),
                        ));

                        let text_b = if self.show_line_numbers {
                            match line.num_b {
                                Some(n) => format!("{:>4} │ {}", n, line.text_b),
                                None => "     │".to_string(),
                            }
                        } else {
                            line.text_b.clone()
                        };
                        cols[1].add(egui::Label::new(
                            RichText::new(text_b).font(font.clone()).background_color(bg_b),
                        ));
                    }

                    if is_3way {
                        let text_c = if self.show_line_numbers {
                            match line.num_c {
                                Some(n) => format!("{:>4} │ {}", n, line.text_c),
                                None => "     │".to_string(),
                            }
                        } else {
                            line.text_c.clone()
                        };
                        cols[2].add(egui::Label::new(
                            RichText::new(text_c).font(font.clone()).background_color(bg_c),
                        ));
                    }
                });
            }
        });
    }
}

/// ChangedAB 라인에 대한 단어 단위 LayoutJob 생성.
/// is_a_side=true → A 쪽(삭제된 단어 강조), false → B 쪽(추가된 단어 강조).
fn word_diff_job(
    line_num: Option<usize>,
    show_numbers: bool,
    font: &FontId,
    text_a: &str,
    text_b: &str,
    is_a_side: bool,
    line_bg: Color32,
    word_bg: Color32,
) -> LayoutJob {
    use similar::{ChangeTag, TextDiff};

    let mut job = LayoutJob::default();

    if show_numbers {
        let prefix = match line_num {
            Some(n) => format!("{:>4} │ ", n),
            None => "     │".to_string(),
        };
        job.append(
            &prefix,
            0.0,
            egui::TextFormat { font_id: font.clone(), background: line_bg, ..Default::default() },
        );
    }

    let diff = TextDiff::from_words(text_a, text_b);
    for change in diff.iter_all_changes() {
        let (include, bg) = match (change.tag(), is_a_side) {
            (ChangeTag::Equal, _) => (true, line_bg),
            (ChangeTag::Delete, true) => (true, word_bg),
            (ChangeTag::Insert, false) => (true, word_bg),
            _ => (false, Color32::TRANSPARENT),
        };
        if include {
            job.append(
                change.value(),
                0.0,
                egui::TextFormat { font_id: font.clone(), background: bg, ..Default::default() },
            );
        }
    }

    job
}

fn build_diff_lines(
    a: Option<&SourceData>,
    b: Option<&SourceData>,
    _c: Option<&SourceData>,
    diff_ab: &DiffList,
) -> Vec<DiffLine> {
    let lines_a = a.map(|s| s.lines.as_slice()).unwrap_or(&[]);
    let lines_b = b.map(|s| s.lines.as_slice()).unwrap_or(&[]);
    let mut result = Vec::new();
    let mut ia = 0usize;
    let mut ib = 0usize;

    for d in diff_ab.iter() {
        for _ in 0..d.nof_equals {
            result.push(DiffLine {
                num_a: Some(ia + 1),
                num_b: Some(ib + 1),
                num_c: None,
                text_a: lines_a.get(ia).map(|l| l.get_line().to_string()).unwrap_or_default(),
                text_b: lines_b.get(ib).map(|l| l.get_line().to_string()).unwrap_or_default(),
                text_c: String::new(),
                kind: LineKind::Equal,
            });
            ia += 1;
            ib += 1;
        }

        let max_diff = d.diff1.max(d.diff2);
        for i in 0..max_diff {
            let (num_a, text_a, num_b, text_b, kind) = match (i < d.diff1, i < d.diff2) {
                (true, true) => (
                    Some(ia + 1),
                    lines_a.get(ia).map(|l| l.get_line().to_string()).unwrap_or_default(),
                    Some(ib + 1),
                    lines_b.get(ib).map(|l| l.get_line().to_string()).unwrap_or_default(),
                    LineKind::ChangedAB,
                ),
                (true, false) => (
                    Some(ia + 1),
                    lines_a.get(ia).map(|l| l.get_line().to_string()).unwrap_or_default(),
                    None,
                    String::new(),
                    LineKind::OnlyA,
                ),
                (false, true) => (
                    None,
                    String::new(),
                    Some(ib + 1),
                    lines_b.get(ib).map(|l| l.get_line().to_string()).unwrap_or_default(),
                    LineKind::OnlyB,
                ),
                (false, false) => unreachable!(),
            };
            result.push(DiffLine {
                num_a,
                num_b,
                num_c: None,
                text_a,
                text_b,
                text_c: String::new(),
                kind,
            });
            if i < d.diff1 { ia += 1; }
            if i < d.diff2 { ib += 1; }
        }
    }

    result
}

fn compute_stats(diff: &DiffList) -> DiffStats {
    let mut s = DiffStats::default();
    for d in diff.iter() {
        match (d.diff1, d.diff2) {
            (0, b) if b > 0 => s.additions += b,
            (a, 0) if a > 0 => s.deletions += a,
            (a, b) if a > 0 && b > 0 => s.changes += a.max(b),
            _ => {}
        }
    }
    s
}
