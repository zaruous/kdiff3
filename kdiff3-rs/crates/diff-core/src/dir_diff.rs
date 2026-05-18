use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Result;

/// 단일 항목의 파일 시스템 메타데이터
#[derive(Debug, Clone)]
pub struct EntryMeta {
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub is_dir: bool,
}

/// 디렉토리 항목의 비교 상태
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryStatus {
    Equal,
    OnlyInA,
    OnlyInB,
    OnlyInC,
    Modified,        // 2-way: A ≠ B
    BModified,       // 3-way: A ≠ B, A = C
    CModified,       // 3-way: A = B, A ≠ C
    BothModified,    // 3-way: A ≠ B, A ≠ C, B = C (자동 해결 가능)
    Conflict,        // 3-way: A ≠ B, A ≠ C, B ≠ C
}

impl EntryStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Equal => "  =",
            Self::OnlyInA => "← A",
            Self::OnlyInB => "B →",
            Self::OnlyInC => "C ↓",
            Self::Modified => " ≠ ",
            Self::BModified => " B~",
            Self::CModified => " C~",
            Self::BothModified => "BC~",
            Self::Conflict => " !! ",
        }
    }

    pub fn is_changed(&self) -> bool {
        !matches!(self, Self::Equal)
    }

    pub fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict)
    }
}

/// 디렉토리 비교의 단일 항목
#[derive(Debug, Clone)]
pub struct DirDiffEntry {
    pub rel_path: PathBuf,
    pub status: EntryStatus,
    pub meta_a: Option<EntryMeta>,
    pub meta_b: Option<EntryMeta>,
    pub meta_c: Option<EntryMeta>,
    pub is_dir: bool,
}

impl DirDiffEntry {
    pub fn abs_path_a(&self, base: &Path) -> Option<PathBuf> {
        self.meta_a.as_ref().map(|_| base.join(&self.rel_path))
    }

    pub fn abs_path_b(&self, base: &Path) -> Option<PathBuf> {
        self.meta_b.as_ref().map(|_| base.join(&self.rel_path))
    }

    pub fn abs_path_c(&self, base: &Path) -> Option<PathBuf> {
        self.meta_c.as_ref().map(|_| base.join(&self.rel_path))
    }
}

/// 디렉토리 비교 결과 전체
pub struct DirDiff {
    pub entries: Vec<DirDiffEntry>,
    pub base_a: PathBuf,
    pub base_b: PathBuf,
    pub base_c: Option<PathBuf>,
}

impl DirDiff {
    /// 2-way 디렉토리 비교
    pub fn compare_twoway(path_a: &Path, path_b: &Path) -> Result<Self> {
        let map_a = collect_entries(path_a)?;
        let map_b = collect_entries(path_b)?;

        let all_keys: std::collections::BTreeSet<_> =
            map_a.keys().chain(map_b.keys()).cloned().collect();

        let mut entries = Vec::new();
        for rel in all_keys {
            let ma = map_a.get(&rel);
            let mb = map_b.get(&rel);
            let is_dir = ma.map(|m| m.is_dir).or(mb.map(|m| m.is_dir)).unwrap_or(false);

            let status = match (ma, mb) {
                (Some(_), None) => EntryStatus::OnlyInA,
                (None, Some(_)) => EntryStatus::OnlyInB,
                (Some(a), Some(b)) => {
                    if is_dir || content_equal(a, b, &path_a.join(&rel), &path_b.join(&rel)) {
                        EntryStatus::Equal
                    } else {
                        EntryStatus::Modified
                    }
                }
                (None, None) => unreachable!(),
            };

            entries.push(DirDiffEntry {
                rel_path: rel,
                status,
                meta_a: ma.cloned(),
                meta_b: mb.cloned(),
                meta_c: None,
                is_dir,
            });
        }

        Ok(Self {
            entries,
            base_a: path_a.to_path_buf(),
            base_b: path_b.to_path_buf(),
            base_c: None,
        })
    }

    /// 3-way 디렉토리 비교
    pub fn compare_threeway(path_a: &Path, path_b: &Path, path_c: &Path) -> Result<Self> {
        let map_a = collect_entries(path_a)?;
        let map_b = collect_entries(path_b)?;
        let map_c = collect_entries(path_c)?;

        let all_keys: std::collections::BTreeSet<_> = map_a
            .keys()
            .chain(map_b.keys())
            .chain(map_c.keys())
            .cloned()
            .collect();

        let mut entries = Vec::new();
        for rel in all_keys {
            let ma = map_a.get(&rel);
            let mb = map_b.get(&rel);
            let mc = map_c.get(&rel);
            let is_dir = [ma, mb, mc]
                .iter()
                .filter_map(|m| m.map(|m| m.is_dir))
                .next()
                .unwrap_or(false);

            let pa = path_a.join(&rel);
            let pb = path_b.join(&rel);
            let pc = path_c.join(&rel);

            let ab_eq = ma.zip(mb).map(|(a, b)| content_equal(a, b, &pa, &pb)).unwrap_or(false);
            let ac_eq = ma.zip(mc).map(|(a, c)| content_equal(a, c, &pa, &pc)).unwrap_or(false);
            let bc_eq = mb.zip(mc).map(|(b, c)| content_equal(b, c, &pb, &pc)).unwrap_or(false);

            let status = match (ma.is_some(), mb.is_some(), mc.is_some()) {
                (true, false, false) => EntryStatus::OnlyInA,
                (false, true, false) => EntryStatus::OnlyInB,
                (false, false, true) => EntryStatus::OnlyInC,
                (true, true, false) => {
                    if ab_eq { EntryStatus::Equal } else { EntryStatus::Modified }
                }
                (true, false, true) => {
                    if ac_eq { EntryStatus::Equal } else { EntryStatus::CModified }
                }
                (false, true, true) => {
                    if bc_eq { EntryStatus::OnlyInB } else { EntryStatus::Conflict }
                }
                (true, true, true) => {
                    if is_dir || (ab_eq && ac_eq) {
                        EntryStatus::Equal
                    } else if ab_eq && !ac_eq {
                        EntryStatus::CModified
                    } else if !ab_eq && ac_eq {
                        EntryStatus::BModified
                    } else if bc_eq {
                        EntryStatus::BothModified
                    } else {
                        EntryStatus::Conflict
                    }
                }
                (false, false, false) => unreachable!(),
            };

            entries.push(DirDiffEntry {
                rel_path: rel,
                status,
                meta_a: ma.cloned(),
                meta_b: mb.cloned(),
                meta_c: mc.cloned(),
                is_dir,
            });
        }

        Ok(Self {
            entries,
            base_a: path_a.to_path_buf(),
            base_b: path_b.to_path_buf(),
            base_c: Some(path_c.to_path_buf()),
        })
    }

    pub fn changed_count(&self) -> usize {
        self.entries.iter().filter(|e| e.status.is_changed()).count()
    }

    pub fn conflict_count(&self) -> usize {
        self.entries.iter().filter(|e| e.status.is_conflict()).count()
    }

    pub fn is_threeway(&self) -> bool {
        self.base_c.is_some()
    }
}

// ── 내부 헬퍼 ────────────────────────────────────────────────────────────────

fn collect_entries(base: &Path) -> Result<BTreeMap<PathBuf, EntryMeta>> {
    let mut map = BTreeMap::new();
    collect_recursive(base, base, &mut map)?;
    Ok(map)
}

fn collect_recursive(
    base: &Path,
    dir: &Path,
    map: &mut BTreeMap<PathBuf, EntryMeta>,
) -> Result<()> {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        let Ok(rel) = path.strip_prefix(base) else { continue };
        let Ok(meta) = entry.metadata() else { continue };

        map.insert(
            rel.to_path_buf(),
            EntryMeta {
                size: meta.len(),
                modified: meta.modified().ok(),
                is_dir: meta.is_dir(),
            },
        );

        if meta.is_dir() {
            collect_recursive(base, &path, map)?;
        }
    }
    Ok(())
}

/// 두 파일이 동일한 내용인지 확인.
/// 크기 다르면 즉시 false, 크기 같으면 mtime 비교 후 필요시 실제 내용 비교.
fn content_equal(a: &EntryMeta, b: &EntryMeta, path_a: &Path, path_b: &Path) -> bool {
    if a.is_dir && b.is_dir {
        return true; // 디렉토리는 항목 수준에서만 비교
    }
    if a.size != b.size {
        return false;
    }
    // mtime이 같으면 내용도 같다고 간주 (빠른 경로)
    if let (Some(ta), Some(tb)) = (a.modified, b.modified) {
        if ta == tb {
            return true;
        }
    }
    // 실제 내용 비교
    match (std::fs::read(path_a), std::fs::read(path_b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
    }
}

// ── 테스트 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_test_dir(root: &Path, files: &[(&str, &str)]) {
        fs::create_dir_all(root).unwrap();
        for (name, content) in files {
            if let Some(parent) = Path::new(name).parent() {
                fs::create_dir_all(root.join(parent)).unwrap();
            }
            fs::write(root.join(name), content).unwrap();
        }
    }

    #[test]
    fn test_twoway_basic() {
        let tmp = std::env::temp_dir().join("kdiff3_test_twoway");
        let dir_a = tmp.join("a");
        let dir_b = tmp.join("b");
        let _ = fs::remove_dir_all(&tmp);

        make_test_dir(&dir_a, &[("same.txt", "hello"), ("only_a.txt", "aaa")]);
        make_test_dir(&dir_b, &[("same.txt", "hello"), ("only_b.txt", "bbb")]);

        let diff = DirDiff::compare_twoway(&dir_a, &dir_b).unwrap();

        let statuses: std::collections::HashMap<_, _> = diff
            .entries
            .iter()
            .map(|e| (e.rel_path.to_string_lossy().to_string(), e.status.clone()))
            .collect();

        assert_eq!(statuses["same.txt"], EntryStatus::Equal);
        assert_eq!(statuses["only_a.txt"], EntryStatus::OnlyInA);
        assert_eq!(statuses["only_b.txt"], EntryStatus::OnlyInB);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_twoway_modified() {
        let tmp = std::env::temp_dir().join("kdiff3_test_modified");
        let dir_a = tmp.join("a");
        let dir_b = tmp.join("b");
        let _ = fs::remove_dir_all(&tmp);

        make_test_dir(&dir_a, &[("file.txt", "version A")]);
        make_test_dir(&dir_b, &[("file.txt", "version B")]);

        let diff = DirDiff::compare_twoway(&dir_a, &dir_b).unwrap();
        assert_eq!(diff.entries[0].status, EntryStatus::Modified);

        let _ = fs::remove_dir_all(&tmp);
    }
}
