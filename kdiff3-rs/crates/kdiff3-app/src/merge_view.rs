use diff_core::DiffList;
use egui::{Color32, FontId, RichText, ScrollArea, Ui};

const COLOR_CONFLICT: Color32 = Color32::from_rgb(255, 150, 150);
const COLOR_RESOLVED: Color32 = Color32::from_rgb(150, 230, 150);
const COLOR_AUTO: Color32 = Color32::from_rgb(220, 220, 255);

#[derive(Clone, PartialEq, Eq)]
enum LineSource {
    A,
    B,
    C,
    Conflict,
    Resolved(String),
}

#[derive(Clone)]
struct MergeLine {
    content: String,
    source: LineSource,
}

/// 병합 결과 뷰. KDiff3의 MergeResultWindow에 대응.
pub struct MergeView {
    lines: Vec<MergeLine>,
    conflict_count: usize,
    current_conflict: usize,
}

impl MergeView {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            conflict_count: 0,
            current_conflict: 0,
        }
    }

    pub fn set_diff(&mut self, diff_ab: DiffList, _diff_ac: Option<DiffList>) {
        self.lines.clear();
        self.conflict_count = 0;

        for diff in diff_ab.iter() {
            for _ in 0..diff.nof_equals {
                self.lines.push(MergeLine {
                    content: String::new(),
                    source: LineSource::A,
                });
            }
            if diff.diff1 > 0 && diff.diff2 > 0 {
                // 양쪽 다 변경 → 충돌
                self.lines.push(MergeLine {
                    content: format!("<<<<<<< (충돌: A {}줄, B {}줄)", diff.diff1, diff.diff2),
                    source: LineSource::Conflict,
                });
                self.conflict_count += 1;
            } else if diff.diff2 > 0 {
                // B만 변경 → B 적용
                for _ in 0..diff.diff2 {
                    self.lines.push(MergeLine {
                        content: String::new(),
                        source: LineSource::B,
                    });
                }
            }
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let font = FontId::monospace(13.0);

        // 충돌 네비게이션 툴바
        ui.horizontal(|ui| {
            ui.label(format!("충돌: {}/{}", self.current_conflict, self.conflict_count));
            if ui.button("◀ 이전").clicked() {
                if self.current_conflict > 0 { self.current_conflict -= 1; }
            }
            if ui.button("다음 ▶").clicked() {
                if self.current_conflict < self.conflict_count {
                    self.current_conflict += 1;
                }
            }
            ui.separator();
            if ui.button("A 선택").clicked() {
                self.resolve_current(LineSource::A);
            }
            if ui.button("B 선택").clicked() {
                self.resolve_current(LineSource::B);
            }
        });

        ui.separator();

        ScrollArea::vertical().show(ui, |ui| {
            for (i, line) in self.lines.iter().enumerate() {
                let (bg, prefix) = match &line.source {
                    LineSource::Conflict => (COLOR_CONFLICT, "!! "),
                    LineSource::Resolved(_) => (COLOR_RESOLVED, "✓  "),
                    LineSource::B => (COLOR_AUTO, "B  "),
                    LineSource::A => (Color32::TRANSPARENT, "A  "),
                    LineSource::C => (Color32::TRANSPARENT, "C  "),
                };

                let text = format!("{:>4} │ {}{}", i + 1, prefix, line.content);
                ui.add(egui::Label::new(
                    RichText::new(text)
                        .font(font.clone())
                        .background_color(bg),
                ));
            }
        });
    }

    fn resolve_current(&mut self, _source: LineSource) {
        // 현재 충돌 블록을 선택한 소스로 해결
        // 실제 구현에서는 conflict 인덱스 추적 필요
    }

    pub fn conflict_count(&self) -> usize {
        self.conflict_count
    }
}
