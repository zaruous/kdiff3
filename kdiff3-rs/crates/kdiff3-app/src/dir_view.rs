use diff_core::{DirDiff, DirDiffEntry, EntryStatus};
use egui::{Color32, FontId, RichText, ScrollArea, Ui};

const COLOR_ONLY_A: Color32 = Color32::from_rgb(255, 200, 200);
const COLOR_ONLY_B: Color32 = Color32::from_rgb(200, 225, 255);
const COLOR_ONLY_C: Color32 = Color32::from_rgb(200, 255, 200);
const COLOR_MODIFIED: Color32 = Color32::from_rgb(255, 240, 180);
const COLOR_CONFLICT: Color32 = Color32::from_rgb(255, 150, 150);

/// 파일을 클릭했을 때 파일 diff로 전환 요청
#[derive(Debug, Clone)]
pub struct OpenFileRequest {
    pub path_a: String,
    pub path_b: String,
    pub path_c: Option<String>,
}

/// 디렉토리 비교 뷰. KDiff3의 DirectoryMergeWindow에 대응.
pub struct DirView {
    dir_diff: Option<DirDiff>,
    show_equal: bool,
    filter: String,
    pub open_request: Option<OpenFileRequest>,
    sort_by: SortKey,
    sort_asc: bool,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum SortKey {
    Path,
    Status,
    SizeA,
    SizeB,
}

impl DirView {
    pub fn new() -> Self {
        Self {
            dir_diff: None,
            show_equal: false,
            filter: String::new(),
            open_request: None,
            sort_by: SortKey::Path,
            sort_asc: true,
        }
    }

    pub fn set_diff(&mut self, diff: DirDiff) {
        self.dir_diff = Some(diff);
        self.open_request = None;
    }

    pub fn show(&mut self, ui: &mut Ui) {
        self.open_request = None; // 매 프레임 초기화 (중복 요청 방지)

        // ── 툴바 ──────────────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_equal, "동일 파일 표시");
            ui.separator();
            ui.label("🔍");
            ui.add_sized(
                [150.0, 20.0],
                egui::TextEdit::singleline(&mut self.filter).hint_text("파일명 필터"),
            );
            ui.separator();

            if let Some(diff) = &self.dir_diff {
                let changed = diff.changed_count();
                let conflicts = diff.conflict_count();
                let total = diff.entries.len();
                ui.label(format!("전체: {total}"));
                ui.separator();
                ui.label(
                    RichText::new(format!("변경: {changed}"))
                        .color(if changed > 0 { Color32::YELLOW } else { Color32::GRAY }),
                );
                if conflicts > 0 {
                    ui.separator();
                    ui.label(RichText::new(format!("충돌: {conflicts}")).color(Color32::RED));
                }
            }
        });

        ui.separator();

        let Some(diff) = &self.dir_diff else {
            ui.centered_and_justified(|ui| {
                ui.label("디렉토리 경로를 입력하고 '폴더 비교' 버튼을 누르세요.");
            });
            return;
        };

        let is_3way = diff.is_threeway();
        let base_a = diff.base_a.clone();
        let base_b = diff.base_b.clone();
        let base_c = diff.base_c.clone();
        let font = FontId::monospace(13.0);

        // ── 헤더 행 (정렬 버튼) ───────────────────────────────────────────────
        // self.dir_diff의 불변 borrow와 self.sort_* 가변 접근 충돌을 피하기 위해
        // 클릭 결과를 로컬 변수로 수집 후 적용
        let cur_key = self.sort_by;
        let cur_asc = self.sort_asc;
        let mut clicked_key: Option<SortKey> = None;

        ui.horizontal(|ui| {
            if sort_btn(ui, "상태", SortKey::Status, cur_key, cur_asc, 50.0) {
                clicked_key = Some(SortKey::Status);
            }
            if sort_btn(ui, "경로", SortKey::Path, cur_key, cur_asc, 320.0) {
                clicked_key = Some(SortKey::Path);
            }
            if sort_btn(ui, "크기 A", SortKey::SizeA, cur_key, cur_asc, 70.0) {
                clicked_key = Some(SortKey::SizeA);
            }
            if sort_btn(ui, "크기 B", SortKey::SizeB, cur_key, cur_asc, 70.0) {
                clicked_key = Some(SortKey::SizeB);
            }
            if is_3way {
                ui.add_sized([70.0, 20.0], egui::Label::new(RichText::new("크기 C").strong()));
            }
            ui.add_sized([100.0, 20.0], egui::Label::new(RichText::new("날짜 A").strong()));
        });

        if let Some(key) = clicked_key {
            if self.sort_by == key {
                self.sort_asc = !self.sort_asc;
            } else {
                self.sort_by = key;
                self.sort_asc = true;
            }
        }

        ui.separator();

        // ── 항목 정렬 ─────────────────────────────────────────────────────────
        let mut entries: Vec<&DirDiffEntry> = diff
            .entries
            .iter()
            .filter(|e| {
                if !self.show_equal && e.status == EntryStatus::Equal {
                    return false;
                }
                if !self.filter.is_empty() {
                    let p = e.rel_path.to_string_lossy().to_lowercase();
                    if !p.contains(&self.filter.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .collect();

        match self.sort_by {
            SortKey::Path => entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path)),
            SortKey::Status => entries.sort_by(|a, b| {
                format!("{:?}", a.status).cmp(&format!("{:?}", b.status))
            }),
            SortKey::SizeA => entries.sort_by(|a, b| {
                a.meta_a.as_ref().map(|m| m.size).unwrap_or(0)
                    .cmp(&b.meta_a.as_ref().map(|m| m.size).unwrap_or(0))
            }),
            SortKey::SizeB => entries.sort_by(|a, b| {
                a.meta_b.as_ref().map(|m| m.size).unwrap_or(0)
                    .cmp(&b.meta_b.as_ref().map(|m| m.size).unwrap_or(0))
            }),
        }
        if !self.sort_asc {
            entries.reverse();
        }

        // ── 항목 목록 ─────────────────────────────────────────────────────────
        let mut pending_open: Option<OpenFileRequest> = None;

        ScrollArea::vertical().id_source("dir_scroll").show(ui, |ui| {
            for entry in &entries {
                let bg = status_color(&entry.status);
                let icon = if entry.is_dir { "📁" } else { "📄" };
                let sym = entry.status.symbol();
                let path_str = entry.rel_path.to_string_lossy();
                let size_a = entry.meta_a.as_ref().map(|m| fmt_size(m.size)).unwrap_or_default();
                let size_b = entry.meta_b.as_ref().map(|m| fmt_size(m.size)).unwrap_or_default();
                let size_c = entry.meta_c.as_ref().map(|m| fmt_size(m.size)).unwrap_or_default();
                let date_a = entry
                    .meta_a
                    .as_ref()
                    .and_then(|m| m.modified)
                    .map(fmt_time)
                    .unwrap_or_default();

                ui.horizontal(|ui| {
                    ui.add_sized(
                        [50.0, 18.0],
                        egui::Label::new(
                            RichText::new(sym).font(font.clone()).background_color(bg),
                        ),
                    );

                    let path_label = egui::Label::new(
                        RichText::new(format!("{icon} {path_str}"))
                            .font(font.clone())
                            .background_color(bg),
                    )
                    .sense(egui::Sense::click());

                    let resp = ui.add_sized([320.0, 18.0], path_label);
                    if resp.clicked() && !entry.is_dir {
                        let pa = entry
                            .abs_path_a(&base_a)
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let pb = entry
                            .abs_path_b(&base_b)
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let pc = base_c.as_ref().and_then(|bc| {
                            entry.abs_path_c(bc).map(|p| p.to_string_lossy().to_string())
                        });
                        pending_open = Some(OpenFileRequest { path_a: pa, path_b: pb, path_c: pc });
                    }
                    if resp.hovered() {
                        resp.on_hover_text(format!(
                            "클릭하면 파일 diff를 엽니다\n{path_str}"
                        ));
                    }

                    ui.add_sized(
                        [70.0, 18.0],
                        egui::Label::new(RichText::new(&size_a).font(font.clone())),
                    );
                    ui.add_sized(
                        [70.0, 18.0],
                        egui::Label::new(RichText::new(&size_b).font(font.clone())),
                    );
                    if is_3way {
                        ui.add_sized(
                            [70.0, 18.0],
                            egui::Label::new(RichText::new(&size_c).font(font.clone())),
                        );
                    }
                    ui.add_sized(
                        [100.0, 18.0],
                        egui::Label::new(RichText::new(&date_a).font(font.clone()).color(Color32::GRAY)),
                    );
                });
            }
        });

        self.open_request = pending_open;
    }

}

/// 정렬 헤더 버튼. 클릭 시 true 반환.
fn sort_btn(
    ui: &mut Ui,
    label: &str,
    key: SortKey,
    cur_key: SortKey,
    asc: bool,
    width: f32,
) -> bool {
    let active = cur_key == key;
    let arrow = if active { if asc { " ▲" } else { " ▼" } } else { "" };
    let text = RichText::new(format!("{label}{arrow}")).strong();
    ui.add_sized([width, 20.0], egui::Button::new(text).frame(false))
        .clicked()
}

fn status_color(s: &EntryStatus) -> Color32 {
    match s {
        EntryStatus::Equal => Color32::TRANSPARENT,
        EntryStatus::OnlyInA => COLOR_ONLY_A,
        EntryStatus::OnlyInB => COLOR_ONLY_B,
        EntryStatus::OnlyInC => COLOR_ONLY_C,
        EntryStatus::Modified | EntryStatus::BModified | EntryStatus::CModified => COLOR_MODIFIED,
        EntryStatus::BothModified | EntryStatus::Conflict => COLOR_CONFLICT,
    }
}

fn fmt_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn fmt_time(t: std::time::SystemTime) -> String {
    use std::time::UNIX_EPOCH;
    let secs = t.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (y, mo, d, h, mi) = epoch_to_ymd(secs);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}")
}

fn epoch_to_ymd(secs: u64) -> (u32, u32, u32, u32, u32) {
    let secs_per_day = 86400u64;
    let days = secs / secs_per_day;
    let time_of_day = secs % secs_per_day;
    let h = (time_of_day / 3600) as u32;
    let mi = ((time_of_day % 3600) / 60) as u32;

    // 그레고리력 변환 (Zeller 공식 단순화)
    let z = days + 719468;
    let era = z / 146097;
    let doe = z % 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if mo <= 2 { y + 1 } else { y } as u32;
    (y, mo, d, h, mi)
}
