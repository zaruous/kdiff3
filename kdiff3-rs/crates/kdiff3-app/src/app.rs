use std::path::Path;

use diff_core::{DirDiff, DiffList, DiffOptions, Merger, ResolvedChoice, SourceData};
use eframe::egui;

use crate::diff_view::DiffView;
use crate::dir_view::DirView;
use crate::merge_view::MergeView;

#[derive(PartialEq, Eq)]
enum ActiveTab {
    Diff,
    Merge,
    Dir,
}

/// 메인 앱 상태. KDiff3의 KDiff3App에 대응.
pub struct KDiff3App {
    // 파일 비교용 경로 입력
    path_a: String,
    path_b: String,
    path_c: String,

    // 디렉토리 비교용 경로 입력
    dir_path_a: String,
    dir_path_b: String,
    dir_path_c: String,

    source_a: Option<SourceData>,
    source_b: Option<SourceData>,
    source_c: Option<SourceData>,

    diff_ab: Option<DiffList>,
    diff_ac: Option<DiffList>,

    diff_view: DiffView,
    merge_view: MergeView,
    dir_view: DirView,

    active_tab: ActiveTab,
    status: String,
    options: DiffOptions,
}

impl KDiff3App {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        file1: Option<String>,
        file2: Option<String>,
        file3: Option<String>,
        _output: Option<String>,
    ) -> Self {
        let mut app = Self {
            path_a: file1.clone().unwrap_or_default(),
            path_b: file2.clone().unwrap_or_default(),
            path_c: file3.clone().unwrap_or_default(),
            dir_path_a: String::new(),
            dir_path_b: String::new(),
            dir_path_c: String::new(),
            source_a: None,
            source_b: None,
            source_c: None,
            diff_ab: None,
            diff_ac: None,
            diff_view: DiffView::new(),
            merge_view: MergeView::new(),
            dir_view: DirView::new(),
            active_tab: ActiveTab::Diff,
            status: "파일 경로를 입력하고 '비교' 버튼을 누르세요.".to_string(),
            options: DiffOptions::default(),
        };

        if file1.is_some() || file2.is_some() {
            app.load_and_diff();
        }

        app
    }

    // ── 파일 비교 ─────────────────────────────────────────────────────────────

    fn load_and_diff(&mut self) {
        self.source_a = None;
        self.source_b = None;
        self.source_c = None;

        if !self.path_a.trim().is_empty() {
            match SourceData::from_file(Path::new(self.path_a.trim())) {
                Ok(s) => self.source_a = Some(s),
                Err(e) => { self.status = format!("A 파일 오류: {e}"); return; }
            }
        }
        if !self.path_b.trim().is_empty() {
            match SourceData::from_file(Path::new(self.path_b.trim())) {
                Ok(s) => self.source_b = Some(s),
                Err(e) => { self.status = format!("B 파일 오류: {e}"); return; }
            }
        }
        if !self.path_c.trim().is_empty() {
            match SourceData::from_file(Path::new(self.path_c.trim())) {
                Ok(s) => self.source_c = Some(s),
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

        let diff_ab = DiffList::from_lines_with_options(&a.lines, &b.lines, &self.options);
        let diff_ac = self.source_c.as_ref().map(|c| DiffList::from_lines_with_options(&a.lines, &c.lines, &self.options));

        self.diff_view.set_sources(
            self.source_a.clone(),
            self.source_b.clone(),
            self.source_c.clone(),
            diff_ab.clone(),
            diff_ac.clone(),
        );

        let merger = if let (Some(ac), Some(c)) = (&diff_ac, &self.source_c) {
            Merger::from_threeway(&diff_ab, ac, &b.lines, &c.lines)
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

        let changes = diff_ab.iter().filter(|d| d.diff1 > 0 || d.diff2 > 0).count();
        self.status = format!(
            "변경 블록: {changes}개{}",
            if conflicts > 0 { format!("  ★ 충돌: {conflicts}개") } else { String::new() }
        );

        self.diff_ab = Some(diff_ab);
        self.diff_ac = diff_ac;
    }

    // ── 디렉토리 비교 ─────────────────────────────────────────────────────────

    fn run_dir_diff(&mut self) {
        let pa = self.dir_path_a.trim();
        let pb = self.dir_path_b.trim();
        let pc = self.dir_path_c.trim();

        if pa.is_empty() || pb.is_empty() {
            self.status = "디렉토리 A, B 경로가 필요합니다.".to_string();
            return;
        }

        let result = if pc.is_empty() {
            DirDiff::compare_twoway(Path::new(pa), Path::new(pb))
        } else {
            DirDiff::compare_threeway(Path::new(pa), Path::new(pb), Path::new(pc))
        };

        match result {
            Ok(diff) => {
                let changed = diff.changed_count();
                let conflicts = diff.conflict_count();
                self.status = format!(
                    "디렉토리: 전체 {}개  변경 {}개{}",
                    diff.entries.len(),
                    changed,
                    if conflicts > 0 { format!("  ★ 충돌: {conflicts}개") } else { String::new() }
                );
                self.dir_view.set_diff(diff);
            }
            Err(e) => {
                self.status = format!("디렉토리 비교 오류: {e}");
            }
        }
    }

    /// DirView에서 파일 클릭 요청을 받아 파일 diff 탭으로 전환
    fn handle_open_request(&mut self) {
        let Some(req) = self.dir_view.open_request.take() else { return };

        self.path_a = req.path_a;
        self.path_b = req.path_b;
        self.path_c = req.path_c.unwrap_or_default();

        self.load_and_diff();
        self.active_tab = ActiveTab::Diff;
    }

    // ── UI ────────────────────────────────────────────────────────────────────

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        match self.active_tab {
            ActiveTab::Diff | ActiveTab::Merge => {
                // 파일 경로 입력 행
                ui.horizontal(|ui| {
                    ui.label("A:");
                    ui.add_sized(
                        [190.0, 20.0],
                        egui::TextEdit::singleline(&mut self.path_a).hint_text("파일 A (base)"),
                    );
                    ui.label("B:");
                    ui.add_sized(
                        [190.0, 20.0],
                        egui::TextEdit::singleline(&mut self.path_b).hint_text("파일 B"),
                    );
                    ui.label("C:");
                    ui.add_sized(
                        [150.0, 20.0],
                        egui::TextEdit::singleline(&mut self.path_c).hint_text("파일 C (선택)"),
                    );
                    if ui.button("비교").clicked() {
                        self.load_and_diff();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.options.ignore_whitespace, "공백 무시");
                    ui.checkbox(&mut self.options.ignore_case, "대소문자 무시");
                });
            }
            ActiveTab::Dir => {
                // 디렉토리 경로 입력 행
                ui.horizontal(|ui| {
                    ui.label("Dir A:");
                    ui.add_sized(
                        [190.0, 20.0],
                        egui::TextEdit::singleline(&mut self.dir_path_a).hint_text("디렉토리 A"),
                    );
                    ui.label("Dir B:");
                    ui.add_sized(
                        [190.0, 20.0],
                        egui::TextEdit::singleline(&mut self.dir_path_b).hint_text("디렉토리 B"),
                    );
                    ui.label("Dir C:");
                    ui.add_sized(
                        [150.0, 20.0],
                        egui::TextEdit::singleline(&mut self.dir_path_c).hint_text("선택"),
                    );
                    if ui.button("폴더 비교").clicked() {
                        self.run_dir_diff();
                    }
                });
            }
        }

        // 탭 / 상태 행
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_tab, ActiveTab::Diff, "📄 파일 비교");
            ui.selectable_value(&mut self.active_tab, ActiveTab::Merge, "🔀 병합");
            ui.selectable_value(&mut self.active_tab, ActiveTab::Dir, "📁 폴더 비교");
            ui.separator();
            ui.label(egui::RichText::new(&self.status).color(egui::Color32::GRAY));
        });
    }
}

impl eframe::App for KDiff3App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // DirView에서 파일 열기 요청 처리 (매 프레임 체크)
        self.handle_open_request();

        // 키보드 단축키 (Merge 탭에서만 활성)
        if self.active_tab == ActiveTab::Merge {
            let (next, prev, res_a, res_b, res_c, save) = ctx.input(|i| {
                let ctrl = i.modifiers.ctrl;
                (
                    ctrl && i.key_pressed(egui::Key::ArrowDown),
                    ctrl && i.key_pressed(egui::Key::ArrowUp),
                    ctrl && i.key_pressed(egui::Key::Num1),
                    ctrl && i.key_pressed(egui::Key::Num2),
                    ctrl && i.key_pressed(egui::Key::Num3),
                    ctrl && i.key_pressed(egui::Key::S),
                )
            });
            if next  { self.merge_view.goto_next_conflict(); }
            if prev  { self.merge_view.goto_prev_conflict(); }
            if res_a { self.merge_view.resolve_current(ResolvedChoice::A); }
            if res_b { self.merge_view.resolve_current(ResolvedChoice::B); }
            if res_c { self.merge_view.resolve_current(ResolvedChoice::C); }
            if save  { self.merge_view.save_output(); }
        }

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            self.show_toolbar(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_tab {
                ActiveTab::Diff => self.diff_view.show(ui),
                ActiveTab::Merge => self.merge_view.show(ui),
                ActiveTab::Dir => self.dir_view.show(ui),
            }
        });
    }
}
