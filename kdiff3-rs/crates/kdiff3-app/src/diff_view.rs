use diff_core::{DiffList, SourceData};
use egui::{Color32, FontId, RichText, ScrollArea, Ui};

const COLOR_EQUAL: Color32 = Color32::TRANSPARENT;
const COLOR_CHANGED_A: Color32 = Color32::from_rgb(255, 200, 200); // 연빨강
const COLOR_CHANGED_B: Color32 = Color32::from_rgb(200, 230, 255); // 연파랑
const COLOR_CHANGED_C: Color32 = Color32::from_rgb(200, 255, 200); // 연초록

/// 라인 상태 (렌더링용)
#[derive(Clone, Copy, PartialEq, Eq)]
enum LineKind {
    Equal,
    OnlyA,
    OnlyB,
    OnlyC,
    Changed,
}

#[derive(Clone)]
struct DiffLine {
    line_a: Option<String>,
    line_b: Option<String>,
    line_c: Option<String>,
    kind: LineKind,
}

/// 비교 텍스트 뷰. KDiff3의 DiffTextWindow에 대응.
pub struct DiffView {
    lines: Vec<DiffLine>,
    scroll_offset: f32,
    show_line_numbers: bool,
    mode: ViewMode,
}

#[derive(PartialEq, Eq)]
enum ViewMode {
    TwoWay,
    ThreeWay,
}

impl DiffView {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            scroll_offset: 0.0,
            show_line_numbers: true,
            mode: ViewMode::TwoWay,
        }
    }

    pub fn set_sources(
        &mut self,
        a: Option<SourceData>,
        b: Option<SourceData>,
        c: Option<SourceData>,
        diff_ab: DiffList,
        diff_ac: Option<DiffList>,
    ) {
        self.mode = if c.is_some() { ViewMode::ThreeWay } else { ViewMode::TwoWay };
        self.lines = build_diff_lines(a.as_ref(), b.as_ref(), c.as_ref(), &diff_ab);
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let is_three_way = self.mode == ViewMode::ThreeWay;

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_line_numbers, "줄 번호");
        });

        ui.separator();

        let font = FontId::monospace(13.0);

        ScrollArea::vertical().show(ui, |ui| {
            // 헤더
            ui.horizontal(|ui| {
                let col_width = if is_three_way { ui.available_width() / 3.0 } else { ui.available_width() / 2.0 };
                ui.add_sized([col_width, 20.0], egui::Label::new(RichText::new("파일 A").strong()));
                ui.add_sized([col_width, 20.0], egui::Label::new(RichText::new("파일 B").strong()));
                if is_three_way {
                    ui.add_sized([col_width, 20.0], egui::Label::new(RichText::new("파일 C").strong()));
                }
            });

            ui.separator();

            for (i, line) in self.lines.iter().enumerate() {
                let col_width = if is_three_way {
                    ui.available_width() / 3.0
                } else {
                    ui.available_width() / 2.0
                };

                ui.horizontal(|ui| {
                    // 라인 번호 + A
                    let bg_a = match line.kind {
                        LineKind::OnlyA | LineKind::Changed => COLOR_CHANGED_A,
                        _ => COLOR_EQUAL,
                    };
                    let text_a = line.line_a.as_deref().unwrap_or("").to_string();
                    let label_a = if self.show_line_numbers {
                        format!("{:>4} │ {}", i + 1, text_a)
                    } else {
                        text_a
                    };
                    ui.add_sized(
                        [col_width, 18.0],
                        egui::Label::new(
                            RichText::new(label_a)
                                .font(font.clone())
                                .background_color(bg_a),
                        ),
                    );

                    // B
                    let bg_b = match line.kind {
                        LineKind::OnlyB | LineKind::Changed => COLOR_CHANGED_B,
                        _ => COLOR_EQUAL,
                    };
                    let text_b = line.line_b.as_deref().unwrap_or("").to_string();
                    let label_b = if self.show_line_numbers {
                        format!("{:>4} │ {}", i + 1, text_b)
                    } else {
                        text_b
                    };
                    ui.add_sized(
                        [col_width, 18.0],
                        egui::Label::new(
                            RichText::new(label_b)
                                .font(font.clone())
                                .background_color(bg_b),
                        ),
                    );

                    // C (3-way일 때만)
                    if is_three_way {
                        let bg_c = match line.kind {
                            LineKind::OnlyC => COLOR_CHANGED_C,
                            _ => COLOR_EQUAL,
                        };
                        let text_c = line.line_c.as_deref().unwrap_or("").to_string();
                        ui.add_sized(
                            [col_width, 18.0],
                            egui::Label::new(
                                RichText::new(text_c)
                                    .font(font.clone())
                                    .background_color(bg_c),
                            ),
                        );
                    }
                });
            }
        });
    }
}

fn build_diff_lines(
    a: Option<&SourceData>,
    b: Option<&SourceData>,
    c: Option<&SourceData>,
    diff_ab: &DiffList,
) -> Vec<DiffLine> {
    let lines_a = a.map(|s| s.lines.clone()).unwrap_or_default();
    let lines_b = b.map(|s| s.lines.clone()).unwrap_or_default();

    let mut result = Vec::new();
    let mut idx_a = 0usize;
    let mut idx_b = 0usize;

    for diff in diff_ab.iter() {
        // 동일 구간
        for _ in 0..diff.nof_equals {
            let la = lines_a.get(idx_a).map(|l| l.get_line().to_string());
            let lb = lines_b.get(idx_b).map(|l| l.get_line().to_string());
            result.push(DiffLine { line_a: la, line_b: lb, line_c: None, kind: LineKind::Equal });
            idx_a += 1;
            idx_b += 1;
        }
        // A만 있는 구간
        for _ in 0..diff.diff1 {
            let la = lines_a.get(idx_a).map(|l| l.get_line().to_string());
            result.push(DiffLine { line_a: la, line_b: None, line_c: None, kind: LineKind::OnlyA });
            idx_a += 1;
        }
        // B만 있는 구간
        for _ in 0..diff.diff2 {
            let lb = lines_b.get(idx_b).map(|l| l.get_line().to_string());
            result.push(DiffLine { line_a: None, line_b: lb, line_c: None, kind: LineKind::OnlyB });
            idx_b += 1;
        }
    }

    result
}
