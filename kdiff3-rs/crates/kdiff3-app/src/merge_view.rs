use std::fs;

use diff_core::{MergeBlock, MergeDetails, Merger, ResolvedChoice, SourceData};
use egui::{Color32, FontId, RichText, ScrollArea, Ui};

const COLOR_CONFLICT_HEADER: Color32 = Color32::from_rgb(220, 80, 80);
const COLOR_A: Color32 = Color32::from_rgb(255, 210, 210);
const COLOR_B: Color32 = Color32::from_rgb(210, 230, 255);
const COLOR_C: Color32 = Color32::from_rgb(210, 255, 210);
const COLOR_AUTO: Color32 = Color32::from_rgb(230, 230, 255);
#[allow(dead_code)]
const COLOR_RESOLVED: Color32 = Color32::from_rgb(180, 240, 180);

/// 병합 결과 뷰. KDiff3의 MergeResultWindow에 대응.
pub struct MergeView {
    merger: Option<Merger>,
    source_a: Option<SourceData>,
    source_b: Option<SourceData>,
    source_c: Option<SourceData>,
    current_conflict: usize, // 현재 포커스된 충돌 블록 인덱스
    output_path: String,
    save_status: String,
}

impl MergeView {
    pub fn new() -> Self {
        Self {
            merger: None,
            source_a: None,
            source_b: None,
            source_c: None,
            current_conflict: 0,
            output_path: String::new(),
            save_status: String::new(),
        }
    }

    pub fn set_data(
        &mut self,
        merger: Merger,
        source_a: Option<SourceData>,
        source_b: Option<SourceData>,
        source_c: Option<SourceData>,
    ) {
        self.merger = Some(merger);
        self.source_a = source_a;
        self.source_b = source_b;
        self.source_c = source_c;
        self.current_conflict = 0;
        self.save_status.clear();
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let font = FontId::monospace(13.0);

        // ── 충돌 네비게이션 툴바 ──────────────────────────────────────────────
        ui.horizontal(|ui| {
            let (total, unresolved) = self
                .merger
                .as_ref()
                .map(|m| (m.conflict_count(), m.unresolved_count()))
                .unwrap_or((0, 0));

            ui.label(
                RichText::new(format!("충돌: {total}개 (미해결: {unresolved}개)"))
                    .color(if unresolved > 0 { Color32::RED } else { Color32::GREEN }),
            );

            ui.separator();

            if ui.button("◀ 이전 충돌").clicked() {
                self.goto_prev_conflict();
            }
            if ui.button("다음 충돌 ▶").clicked() {
                self.goto_next_conflict();
            }

            ui.separator();

            // 현재 포커스된 충돌에 대한 빠른 해결 버튼
            let has_conflict = self
                .merger
                .as_ref()
                .and_then(|m| m.blocks.get(self.current_conflict))
                .map(|b| b.is_unresolved_conflict())
                .unwrap_or(false);

            ui.add_enabled_ui(has_conflict, |ui| {
                if ui.button("A 선택").clicked() {
                    self.resolve_current(ResolvedChoice::A);
                }
                if ui.button("B 선택").clicked() {
                    self.resolve_current(ResolvedChoice::B);
                }
                if ui.button("C 선택").clicked() {
                    self.resolve_current(ResolvedChoice::C);
                }
            });
        });

        ui.separator();

        // ── 저장 영역 ─────────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("출력 파일:");
            ui.add_sized(
                [300.0, 20.0],
                egui::TextEdit::singleline(&mut self.output_path).hint_text("/path/to/output.txt"),
            );
            if ui.button("저장").clicked() {
                self.save_output();
            }
            if !self.save_status.is_empty() {
                ui.label(RichText::new(&self.save_status).color(
                    if self.save_status.starts_with("✓") { Color32::GREEN } else { Color32::RED },
                ));
            }
        });

        ui.separator();

        // ── 병합 결과 스크롤 영역 ────────────────────────────────────────────
        let Some(merger) = &self.merger else {
            ui.centered_and_justified(|ui| {
                ui.label("파일을 열면 병합 결과가 여기에 표시됩니다.");
            });
            return;
        };

        let lines_a = self.source_a.as_ref().map(|s| s.lines.as_slice()).unwrap_or(&[]);
        let lines_b = self.source_b.as_ref().map(|s| s.lines.as_slice()).unwrap_or(&[]);
        let lines_c = self.source_c.as_ref().map(|s| s.lines.as_slice()).unwrap_or(&[]);

        // blocks는 immutable borrow, current_conflict는 별도 관리
        let blocks: Vec<MergeBlock> = merger.blocks.clone();
        let current = self.current_conflict;

        let mut pending_resolve: Option<(usize, ResolvedChoice)> = None;

        ScrollArea::vertical().id_source("merge_scroll").show(ui, |ui| {
            for (idx, block) in blocks.iter().enumerate() {
                let is_current = idx == current;

                match &block.details {
                    MergeDetails::NoChange => {
                        // 동일 라인들: 접혀 있는 형태로 간략 표시
                        let count = block.a_range.len();
                        if count > 6 {
                            ui.add(egui::Label::new(
                                RichText::new(format!("  ... {count}줄 동일 ..."))
                                    .font(font.clone())
                                    .color(Color32::GRAY),
                            ));
                        } else {
                            for i in block.a_range.clone() {
                                if let Some(l) = lines_a.get(i) {
                                    ui.add(egui::Label::new(
                                        RichText::new(format!("{:>4} │ {}", i + 1, l.get_line()))
                                            .font(font.clone()),
                                    ));
                                }
                            }
                        }
                    }

                    MergeDetails::BChanged | MergeDetails::BAdded | MergeDetails::BDeleted => {
                        // B 단독 변경 → 자동 적용
                        render_auto_block(ui, &font, "B", COLOR_AUTO, &block.b_range, lines_b);
                    }

                    MergeDetails::CChanged | MergeDetails::CAdded | MergeDetails::CDeleted => {
                        render_auto_block(ui, &font, "C", COLOR_AUTO, &block.c_range, lines_c);
                    }

                    MergeDetails::BCChangedAndEqual | MergeDetails::BCAddedAndEqual => {
                        render_auto_block(ui, &font, "B=C", COLOR_AUTO, &block.b_range, lines_b);
                    }

                    MergeDetails::BCChanged | MergeDetails::BCAdded => {
                        // 충돌 블록
                        let resolved = block.resolved;
                        render_conflict_block(
                            ui,
                            &font,
                            idx,
                            is_current,
                            resolved,
                            block,
                            lines_a,
                            lines_b,
                            lines_c,
                        );

                        // 해결 버튼 (블록 바로 아래)
                        if resolved.is_none() {
                            ui.horizontal(|ui| {
                                if ui.small_button("→ A").clicked() {
                                    pending_resolve = Some((idx, ResolvedChoice::A));
                                }
                                if ui.small_button("→ B").clicked() {
                                    pending_resolve = Some((idx, ResolvedChoice::B));
                                }
                                if ui.small_button("→ C").clicked() {
                                    pending_resolve = Some((idx, ResolvedChoice::C));
                                }
                            });
                        }
                    }

                    _ => {
                        // BCDeleted 등 기타
                        ui.add(egui::Label::new(
                            RichText::new(format!("[{:?}]", block.details))
                                .font(font.clone())
                                .color(Color32::GRAY),
                        ));
                    }
                }
            }
        });

        if let Some((idx, choice)) = pending_resolve {
            if let Some(m) = &mut self.merger {
                m.resolve(idx, choice);
            }
            self.current_conflict = idx;
        }
    }

    pub fn goto_next_conflict(&mut self) {
        if let Some(m) = &self.merger {
            let start = if self.current_conflict + 1 < m.blocks.len() {
                self.current_conflict + 1
            } else {
                0
            };
            if let Some(next) = m.next_conflict_after(start) {
                self.current_conflict = next;
            } else if let Some(next) = m.next_conflict_after(0) {
                self.current_conflict = next;
            }
        }
    }

    pub fn goto_prev_conflict(&mut self) {
        if let Some(m) = &self.merger {
            if let Some(prev) = m.prev_conflict_before(self.current_conflict) {
                self.current_conflict = prev;
            }
        }
    }

    pub fn resolve_current(&mut self, choice: ResolvedChoice) {
        if let Some(m) = &mut self.merger {
            m.resolve(self.current_conflict, choice);
        }
    }

    pub fn save_output(&mut self) {
        let path = self.output_path.trim().to_string();
        if path.is_empty() {
            self.save_status = "✗ 출력 경로를 입력하세요.".to_string();
            return;
        }

        let (lines_a, lines_b, lines_c) = match (&self.source_a, &self.source_b, &self.source_c) {
            (Some(a), Some(b), c) => (
                a.lines.as_slice(),
                b.lines.as_slice(),
                c.as_ref().map(|c| c.lines.as_slice()).unwrap_or(&[]),
            ),
            _ => {
                self.save_status = "✗ 파일이 로드되지 않았습니다.".to_string();
                return;
            }
        };

        if let Some(m) = &self.merger {
            let output = m.output(lines_a, lines_b, lines_c);
            let content = output.join("\n");
            match fs::write(&path, content) {
                Ok(_) => {
                    self.save_status = format!("✓ 저장 완료: {path}");
                }
                Err(e) => {
                    self.save_status = format!("✗ 저장 실패: {e}");
                }
            }
        }
    }
}

// ── 렌더링 헬퍼 ──────────────────────────────────────────────────────────────

fn render_auto_block(
    ui: &mut Ui,
    font: &FontId,
    label: &str,
    bg: Color32,
    range: &std::ops::Range<usize>,
    lines: &[diff_core::LineData],
) {
    for i in range.clone() {
        if let Some(l) = lines.get(i) {
            ui.add(egui::Label::new(
                RichText::new(format!("{:>4} │[{label}] {}", i + 1, l.get_line()))
                    .font(font.clone())
                    .background_color(bg),
            ));
        }
    }
}

fn render_conflict_block(
    ui: &mut Ui,
    font: &FontId,
    idx: usize,
    is_current: bool,
    resolved: Option<ResolvedChoice>,
    block: &MergeBlock,
    lines_a: &[diff_core::LineData],
    lines_b: &[diff_core::LineData],
    lines_c: &[diff_core::LineData],
) {
    let header_bg = if is_current {
        Color32::from_rgb(180, 50, 50)
    } else {
        COLOR_CONFLICT_HEADER
    };

    let status = match resolved {
        Some(ResolvedChoice::A) => " [해결됨→A]",
        Some(ResolvedChoice::B) => " [해결됨→B]",
        Some(ResolvedChoice::C) => " [해결됨→C]",
        None => " ★ 충돌",
    };

    ui.add(egui::Label::new(
        RichText::new(format!("━━━ 충돌 #{}{status} ━━━", idx + 1))
            .font(font.clone())
            .background_color(header_bg)
            .color(Color32::WHITE)
            .strong(),
    ));

    // A 내용
    ui.add(egui::Label::new(
        RichText::new("  ─── A (base) ───")
            .font(font.clone())
            .background_color(COLOR_A),
    ));
    for i in block.a_range.clone() {
        if let Some(l) = lines_a.get(i) {
            ui.add(egui::Label::new(
                RichText::new(format!("{:>4} │ {}", i + 1, l.get_line()))
                    .font(font.clone())
                    .background_color(COLOR_A),
            ));
        }
    }

    // B 내용
    ui.add(egui::Label::new(
        RichText::new("  ─── B ───")
            .font(font.clone())
            .background_color(COLOR_B),
    ));
    for i in block.b_range.clone() {
        if let Some(l) = lines_b.get(i) {
            ui.add(egui::Label::new(
                RichText::new(format!("{:>4} │ {}", i + 1, l.get_line()))
                    .font(font.clone())
                    .background_color(COLOR_B),
            ));
        }
    }

    // C 내용 (3-way일 때만)
    if !block.c_range.is_empty() {
        ui.add(egui::Label::new(
            RichText::new("  ─── C ───")
                .font(font.clone())
                .background_color(COLOR_C),
        ));
        for i in block.c_range.clone() {
            if let Some(l) = lines_c.get(i) {
                ui.add(egui::Label::new(
                    RichText::new(format!("{:>4} │ {}", i + 1, l.get_line()))
                        .font(font.clone())
                        .background_color(COLOR_C),
                ));
            }
        }
    }
}
