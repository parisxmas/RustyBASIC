use codespan_reporting::files::SimpleFiles;

/// Byte offset into source text.
pub type ByteOffset = usize;

/// A span of source code, represented as a byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: ByteOffset,
    pub end: ByteOffset,
}

impl Span {
    pub fn new(start: ByteOffset, end: ByteOffset) -> Self {
        Self { start, end }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn to_range(self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

impl From<std::ops::Range<usize>> for Span {
    fn from(range: std::ops::Range<usize>) -> Self {
        Span {
            start: range.start,
            end: range.end,
        }
    }
}

impl From<Span> for std::ops::Range<usize> {
    fn from(span: Span) -> Self {
        span.start..span.end
    }
}

/// File id for codespan-reporting.
pub type FileId = usize;

/// Source file database using codespan-reporting.
pub type SourceDb = SimpleFiles<String, String>;
