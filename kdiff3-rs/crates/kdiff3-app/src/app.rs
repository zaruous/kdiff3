use std::path::Path;

use diff_core::{DiffList, Merger, SourceData};
use eframe::egui;

use crate::diff_view::DiffView;
use crate::merge_view::MergeView;

#[derive(PartialEq, Eq)]
enum ActiveTab { Diff, Merge }

/// 메인 앱 상태. KDiff3의 KDiff3App에 대응.
pub struct KDiff3App {
    // 파일 경로 입력 필드
    path_a: String,
    path_b: String,
    path_c: String,

    source_a: Option<SourceData>,
    source_b: Option<SourceData>,
    source_c: Option<SourceData>,

    diff_ab: Option<DiffList>,
    diff_ac: Option<DiffList>,

    diff_view: DiffView,
    merge_view: MergeView,

    active_tab: ActiveTab,
    status: String,
}

impl KDiff3App {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        file1: Option<String>,
        file2: Option<String>,
        file3: Option<String>,
        output: Option<String>,
    ) -> Self {
        let mut app = Self {
            path_a: file1.clone().unwrap_or_default(),
            path_b: file2.clone().unwrap_or_default(),
            path_c: file3.clone().unwrap_or_default(),
            source_a: None,
            source_b: None,
            source_c: None,
            diff_ab: None,
            diff_ac: None,
            diff_view: DiffView::new(),
            merge_view: MergeView::new(),
            active_tab: ActiveTab::Diff,
            status: "파일 경로를 입력하고 '비교' 버튼을 누르세요.".to_string(),
        };

        // CLI 인수로 파일이 주어진 경우 즉시 로드
        if file1.is_some() || file2.is_some() {
            app.load_and_diff();
        }

        let _ = output; // merge_view 내부에서 직접 출력 경로를 관리

        app
    }

    fn load_and_diff(&mut self) {
        // 파일 A 로드
        if !self.path_a.trim().is_empty() {
            match SourceData::from_file(Path::new(self.path_a.trim())) {
                Ok(s) => { self.source_a = Some(s); }
                Err(e) => { self.status = format!("A 파일 오류: {e}"); return; }
            }
        }
        // 파일 B 로드
        if !self.path_b.trim().is_empty() {
            match SourceData::from_file(Path::new(self.path_b.trim())) {
                Ok(s) => { self.source_b = Some(s); }
                Err(e) => { self.status = format!("B 파일 오류: {e}"); return; }
            }
        }
        // 파일 C 로드 (선택)
        if !self.path_c.trim().is_empty() {
            match SourceData::from_file(Path::new(self.path_c.trim())) {
                Ok(s) => { self.source_c = Some(s); }
                Err(e) => { self.status = format!("C 파일 오류: {e}"); return; }
            }
        }

        self.run_diff();
    }

    fn run_diff(&mut self) {
        let (Some(a), Some(b)) = (&self.source_a, &self.source_b) else {
            self.status = "A, B 파일이 모두 필요합니다.".to_string();
            return;
        };

        let diff_ab = DiffList::from_lines(&a.lines, &b.lines);
        let diff_ac = self.source_c.as_ref().map(|c| DiffList::from_lines(&a.lines, &c.lines));

        // diff_view 업데이트
        self.diff_view.set_sources(
            self.source_a.clone(),
            self.source_b.clone(),
            self.source_c.clone(),
            diff_ab.clone(),
            diff_ac.clone(),
        );

        // merger 생성 후 merge_view 업데이트
        let merger = if let Some(ref ac) = diff_ac {
            Merger::from_threeway(&diff_ab, ac, &b.lines, &self.source_c.as_ref().unwrap().lines)
        } else {
            Merger::from_twoway(&diff_ab)
        };

        let conflicts = merger.conflict_count();
        self.merge_view.set_data(
            merger,
            self.source_a.clone(),
            self.source_b.clone(),
            self.source_c.clone(),
        );

        let change_blocks = diff_ab.iter().filter(|d| d.diff1 > 0 || d.diff2 > 0).count();
        self.status = format!(
            "변경 블록: {change_blocks}개{}",
            if conflicts > 0 { format!("  충돌: {conflicts}개") } else { String::new() }
        );

        self.diff_ab = Some(diff_ab);
        self.diff_ac = diff_ac;
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        // ── 파일 경로 입력 행 ─────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("A:");
            ui.add_sized(
                [200.0, 20.0],
                egui::TextEdit::singleline(&mut self.path_a).hint_text("파일 A 경로 (base)"),
            );
            ui.label("B:");
            ui.add_sized(
                [200.0, 20.0],
                egui::TextEdit::singleline(&mut self.path_b).hint_text("파일 B 경로"),
            );
            ui.label("C:");
            ui.add_sized(
                [160.0, 20.0],
                egui::TextEdit::singleline(&mut self.path_c).hint_text("파일 C (선택)"),
            );
            if ui.button("비교").clicked() {
                self.load_and_diff();
            }
        });

        // ── 탭 / 상태 행 ──────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_tab, ActiveTab::Diff, "📄 비교");
            ui.selectable_value(&mut self.active_tab, ActiveTab::Merge, "🔀 병합");
            ui.separator();
            ui.label(egui::RichText::new(&self.status).color(egui::Color32::GRAY));
        });
    }
}

impl eframe::App for KDiff3App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            self.show_toolbar(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_tab {
                ActiveTab::Diff => self.diff_view.show(ui),
                ActiveTab::Merge => self.merge_view.show(ui),
            }
        });
    }
}
