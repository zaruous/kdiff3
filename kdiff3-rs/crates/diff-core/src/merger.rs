use crate::diff::DiffList;

/// 3-way merge 결과 상태. KDiff3의 e_MergeDetails에 대응.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeDetails {
    NoChange,
    BChanged,
    CChanged,
    BCChanged,           // 충돌
    BCChangedAndEqual,   // B==C로 변경 (자동 해결 가능)
    BDeleted,
    CDeleted,
    BCDeleted,
    BChangedCDeleted,    // 충돌
    CChangedBDeleted,    // 충돌
    BAdded,
    CAdded,
    BCAdded,             // 충돌
    BCAddedAndEqual,     // 자동 해결 가능
}

impl MergeDetails {
    pub fn is_conflict(&self) -> bool {
        matches!(
            self,
            MergeDetails::BCChanged
                | MergeDetails::BChangedCDeleted
                | MergeDetails::CChangedBDeleted
                | MergeDetails::BCAdded
        )
    }

    pub fn is_auto_solvable(&self) -> bool {
        matches!(
            self,
            MergeDetails::BCChangedAndEqual | MergeDetails::BCAddedAndEqual | MergeDetails::NoChange
        )
    }
}

/// 병합 결과의 단일 라인
#[derive(Debug, Clone)]
pub struct MergeResult {
    pub line_a: Option<usize>,
    pub line_b: Option<usize>,
    pub line_c: Option<usize>,
    pub details: MergeDetails,
}

impl MergeResult {
    pub fn is_conflict(&self) -> bool {
        self.details.is_conflict()
    }
}

/// 3-way merge 이터레이터. KDiff3의 Merger에 대응.
pub struct Merger {
    diff_ab: DiffList, // A vs B
    diff_ac: DiffList, // A vs C
    results: Vec<MergeResult>,
    current: usize,
}

impl Merger {
    pub fn new(diff_ab: DiffList, diff_ac: DiffList) -> Self {
        let mut m = Self { diff_ab, diff_ac, results: Vec::new(), current: 0 };
        m.compute();
        m
    }

    fn compute(&mut self) {
        // 단순화된 3-way merge 계산
        // A를 기준으로 B, C의 변경을 추적
        let mut line_a = 0usize;
        let mut line_b = 0usize;
        let mut line_c = 0usize;

        for diff in self.diff_ab.iter() {
            // 동일 구간
            for _ in 0..diff.nof_equals {
                self.results.push(MergeResult {
                    line_a: Some(line_a),
                    line_b: Some(line_b),
                    line_c: Some(line_c),
                    details: MergeDetails::NoChange,
                });
                line_a += 1;
                line_b += 1;
                line_c += 1;
            }
            // B에서 변경된 구간
            for i in 0..diff.diff1.max(diff.diff2) {
                let la = if i < diff.diff1 { Some(line_a + i) } else { None };
                let lb = if i < diff.diff2 { Some(line_b + i) } else { None };
                self.results.push(MergeResult {
                    line_a: la,
                    line_b: lb,
                    line_c: None,
                    details: MergeDetails::BChanged,
                });
            }
            line_a += diff.diff1;
            line_b += diff.diff2;
        }

        let _ = line_c; // C 처리는 diff_ac로 추후 확장
    }

    pub fn results(&self) -> &[MergeResult] {
        &self.results
    }

    pub fn conflict_count(&self) -> usize {
        self.results.iter().filter(|r| r.is_conflict()).count()
    }

    pub fn next(&mut self) -> Option<&MergeResult> {
        if self.current < self.results.len() {
            let r = &self.results[self.current];
            self.current += 1;
            Some(r)
        } else {
            None
        }
    }

    pub fn is_end(&self) -> bool {
        self.current >= self.results.len()
    }

    pub fn reset(&mut self) {
        self.current = 0;
    }
}
