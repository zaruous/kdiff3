use std::path::Path;

use diff_core::{DiffList, SourceData};
use eframe::egui;

use crate::diff_view::DiffView;
use crate::merge_view::MergeView;

#[derive(PartialEq, Eq)]
enum ActiveTab {
    Diff,
    Merge,
}

/// 메인 앱 상태. KDiff3의 KDiff3App에 대응.
pub struct KDiff3App {
    source_a: Option<SourceData>,
    source_b: Option<SourceData>,
    source_c: Option<SourceData>,
    output_path: Option<String>,

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
            source_a: None,
            source_b: None,
            source_c: None,
            output_path: output,
            diff_ab: None,
            diff_ac: None,
            diff_view: DiffView::new(),
            merge_view: MergeView::new(),
            active_tab: ActiveTab::Diff,
            status: "파일을 열어주세요.".to_string(),
        };

        if let Some(f) = file1 { app.load_file_a(&f); }
        if let Some(f) = file2 { app.load_file_b(&f); }
        if let Some(f) = file3 { app.load_file_c(&f); }
        app.run_diff();
        app
    }

    pub fn load_file_a(&mut self, path: &str) {
        match SourceData::from_file(Path::new(path)) {
            Ok(s) => { self.source_a = Some(s); }
            Err(e) => { self.status = format!("A 파일 오류: {e}"); }
        }
    }

    pub fn load_file_b(&mut self, path: &str) {
        match SourceData::from_file(Path::new(path)) {
            Ok(s) => { self.source_b = Some(s); }
            Err(e) => { self.status = format!("B 파일 오류: {e}"); }
        }
    }

    pub fn load_file_c(&mut self, path: &str) {
        match SourceData::from_file(Path::new(path)) {
            Ok(s) => { self.source_c = Some(s); }
            Err(e) => { self.status = format!("C 파일 오류: {e}"); }
        }
    }

    pub fn run_diff(&mut self) {
        let (Some(a), Some(b)) = (&self.source_a, &self.source_b) else {
            return;
        };

        let diff_ab = DiffList::from_lines(&a.lines, &b.lines);

        let diff_ac = self.source_c.as_ref().map(|c| DiffList::from_lines(&a.lines, &c.lines));

        self.diff_view.set_sources(
            self.source_a.clone(),
            self.source_b.clone(),
            self.source_c.clone(),
            diff_ab.clone(),
            diff_ac.clone(),
        );
        self.merge_view.set_diff(diff_ab.clone(), diff_ac.clone());

        let changes = diff_ab.iter().filter(|d| d.diff1 > 0 || d.diff2 > 0).count();
        self.status = format!("차이 블록: {changes}개");
        self.diff_ab = Some(diff_ab);
        self.diff_ac = diff_ac;
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("파일 A 열기").clicked() {
                if let Some(path) = rfd_pick_file() {
                    self.load_file_a(&path);
                    self.run_diff();
                }
            }
            if ui.button("파일 B 열기").clicked() {
                if let Some(path) = rfd_pick_file() {
                    self.load_file_b(&path);
                    self.run_diff();
                }
            }
            if ui.button("파일 C 열기").clicked() {
                if let Some(path) = rfd_pick_file() {
                    self.load_file_c(&path);
                    self.run_diff();
                }
            }
            ui.separator();
            ui.selectable_value(&mut self.active_tab, ActiveTab::Diff, "비교");
            ui.selectable_value(&mut self.active_tab, ActiveTab::Merge, "병합");
            ui.separator();
            ui.label(&self.status);
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

fn rfd_pick_file() -> Option<String> {
    // rfd(native file dialog) 없이 단순 구현 — 추후 rfd 크레이트로 교체
    None
}
