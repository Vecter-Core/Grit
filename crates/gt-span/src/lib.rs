#![forbid(unsafe_code)]
//! gt-span (M1): span theo byte và bản đồ nguồn (source map).
//!
//! Offset là `u32` (tệp nguồn tối đa 4 GiB − 1). Cột và dòng tính theo **Unicode scalar value**,
//! không theo byte, 1-based. Xuống dòng là `\n`; `\r\n` được coi là một xuống dòng (`\r` không
//! thuộc nội dung dòng khi lấy `line_text`).

/// Đoạn nguồn nửa mở `[start, end)` theo byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    /// Tạo span. `start > end` là lỗi lập trình trong compiler (ICE) nên panic có chủ ý.
    #[must_use]
    pub fn new(start: u32, end: u32) -> Span {
        assert!(start <= end, "Span::new: start ({start}) > end ({end})");
        Span { start, end }
    }

    #[must_use]
    pub fn len(self) -> u32 {
        self.end - self.start
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Span nhỏ nhất bao cả hai.
    #[must_use]
    pub fn to(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// Vị trí dòng/cột 1-based (cột đếm Unicode scalar value).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

/// Nguồn quá lớn để đánh offset bằng `u32`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceTooLarge {
    pub len: usize,
}

/// Một tệp nguồn đã nạp, kèm bảng đầu dòng.
#[derive(Clone, Debug)]
pub struct SourceFile {
    name: String,
    text: String,
    /// Offset byte đầu mỗi dòng; luôn có phần tử 0.
    line_starts: Vec<u32>,
}

impl SourceFile {
    /// Nạp nguồn. Lỗi nếu dài hơn `u32::MAX` byte.
    pub fn new(name: impl Into<String>, text: String) -> Result<SourceFile, SourceTooLarge> {
        if u32::try_from(text.len()).is_err() {
            return Err(SourceTooLarge { len: text.len() });
        }
        let mut line_starts = vec![0u32];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                // i < len <= u32::MAX nên i + 1 không tràn u32 khi len <= u32::MAX
                line_starts
                    .push(u32::try_from(i + 1).map_err(|_| SourceTooLarge { len: text.len() })?);
            }
        }
        Ok(SourceFile {
            name: name.into(),
            text,
            line_starts,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Độ dài theo byte.
    #[must_use]
    pub fn len(&self) -> u32 {
        // an toàn: đã kiểm trong `new`
        u32::try_from(self.text.len()).unwrap_or(u32::MAX)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Số dòng (tệp rỗng và tệp không có xuống dòng đều là 1 dòng).
    #[must_use]
    pub fn line_count(&self) -> u32 {
        u32::try_from(self.line_starts.len()).unwrap_or(u32::MAX)
    }

    /// Dòng/cột của offset. `None` nếu offset > độ dài hoặc không nằm trên ranh giới ký tự.
    #[must_use]
    pub fn line_col(&self, offset: u32) -> Option<LineCol> {
        if offset > self.len() || !self.text.is_char_boundary(offset as usize) {
            return None;
        }
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1, // i >= 1 vì line_starts[0] == 0 <= offset
        };
        let start = self.line_starts[line_idx] as usize;
        let col = self.text[start..offset as usize].chars().count();
        Some(LineCol {
            line: u32::try_from(line_idx + 1).ok()?,
            col: u32::try_from(col + 1).ok()?,
        })
    }

    /// Văn bản của span; `None` nếu ra ngoài tệp hoặc cắt giữa ký tự.
    #[must_use]
    pub fn snippet(&self, span: Span) -> Option<&str> {
        self.text.get(span.start as usize..span.end as usize)
    }

    /// Nội dung dòng `line` (1-based), không gồm `\n` và `\r` cuối dòng.
    #[must_use]
    pub fn line_text(&self, line: u32) -> Option<&str> {
        let idx = usize::try_from(line.checked_sub(1)?).ok()?;
        let start = *self.line_starts.get(idx)? as usize;
        let end = self
            .line_starts
            .get(idx + 1)
            .map_or(self.text.len(), |&e| e as usize);
        let raw = &self.text[start..end];
        let raw = raw.strip_suffix('\n').unwrap_or(raw);
        Some(raw.strip_suffix('\r').unwrap_or(raw))
    }
}

/// Định danh một tệp trong một chương trình nhiều tệp (ADR-017a). Do `SourceMap` cấp: **hạng của tên tệp trong danh sách đã
/// sắp xếp**, nên KHÔNG phụ thuộc thứ tự đưa tệp vào.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(u32);

impl FileId {
    #[must_use]
    pub fn new(raw: u32) -> FileId {
        FileId(raw)
    }

    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Lỗi dựng `SourceMap`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceMapError {
    /// Hai tệp cùng tên.
    DuplicateName(String),
}

/// Tập tệp của một chương trình, đánh `FileId` theo thứ tự tên (tất định, độc lập thứ tự nhập).
#[derive(Clone, Debug)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    /// Dựng bản đồ; sắp theo tên (so sánh byte). Tên trùng → lỗi.
    pub fn new(mut files: Vec<SourceFile>) -> Result<SourceMap, SourceMapError> {
        files.sort_by(|a, b| a.name().cmp(b.name()));
        for w in files.windows(2) {
            if w[0].name() == w[1].name() {
                return Err(SourceMapError::DuplicateName(w[0].name().to_string()));
            }
        }
        Ok(SourceMap { files })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    #[must_use]
    pub fn get(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.index())
    }

    #[must_use]
    pub fn id_of(&self, name: &str) -> Option<FileId> {
        self.files
            .binary_search_by(|f| f.name().cmp(name))
            .ok()
            .and_then(|i| u32::try_from(i).ok())
            .map(FileId::new)
    }

    /// Duyệt theo `FileId` tăng dần (= theo tên).
    pub fn iter(&self) -> impl Iterator<Item = (FileId, &SourceFile)> {
        self.files
            .iter()
            .enumerate()
            .map(|(i, f)| (FileId::new(u32::try_from(i).unwrap_or(u32::MAX)), f))
    }
}
