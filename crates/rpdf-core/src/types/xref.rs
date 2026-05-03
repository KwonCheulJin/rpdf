use std::collections::BTreeMap;

/// xref 테이블 전체. 객체 번호를 키로, 엔트리를 값으로 갖는다.
///
/// `BTreeMap`을 사용해 객체 번호 순 정렬을 보장한다.
/// incremental update chain 병합 시 `or_insert` 의미론을 적용한다:
/// 최신(현재) 섹션의 엔트리가 이전 섹션의 엔트리보다 우선한다.
///
/// 내부 자료구조(`BTreeMap`)는 `pub(crate)`로 숨기고 메서드로만 접근한다.
/// 향후 자료구조 교체 시 외부 코드에 영향을 주지 않기 위함이다.
/// 추가 메서드(iter, contains 등)는 Task #4에서 실제 필요해지면 추가한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrefTable {
    pub(crate) entries: BTreeMap<u32, XrefEntry>,
}

impl XrefTable {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// 해당 객체 번호의 엔트리가 아직 없을 때만 삽입한다.
    /// (최신 섹션이 먼저 삽입되므로, 이전 섹션 엔트리는 무시된다.)
    pub fn insert_if_absent(&mut self, obj_num: u32, entry: XrefEntry) {
        self.entries.entry(obj_num).or_insert(entry);
    }

    pub fn get(&self, obj_num: u32) -> Option<&XrefEntry> {
        self.entries.get(&obj_num)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 모든 엔트리를 `(obj_num, XrefEntry)` 쌍으로 순회한다 (obj_num 오름차순).
    pub fn iter(&self) -> impl Iterator<Item = (&u32, &XrefEntry)> {
        self.entries.iter()
    }
}

impl Default for XrefTable {
    fn default() -> Self {
        Self::new()
    }
}

/// 개별 xref 엔트리.
///
/// PDF 스펙(ISO 32000) §7.5.4 — Cross-Reference Table
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrefEntry {
    /// 사용 중인 객체: 파일 내 바이트 오프셋과 generation 번호.
    InUse { offset: u64, generation: u16 },
    /// 삭제된(free) 객체: 다음 free 객체 번호와 generation 번호.
    Free {
        next_free_obj_num: u32,
        generation: u16,
    },
    /// 압축 객체 스트림 내 객체 (PDF 1.5+, Task #5에서 처리).
    Compressed { obj_stm_num: u32, index: u32 },
}
