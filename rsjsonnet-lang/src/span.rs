use std::collections::HashMap;
use std::num::NonZeroU64;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpanId(NonZeroU64);

impl SpanId {
    const OFFSET_BITS: u32 = 38;
    const OFFSET_MASK: u64 = (1 << Self::OFFSET_BITS) - 1;
    const LEN_MAX: u64 = (1 << (63 - Self::OFFSET_BITS)) - 1;

    fn expand(self) -> ExpandedSpanId {
        let inner = self.0.get();
        if (inner & (1 << 63)) == 0 {
            let offset = (inner & Self::OFFSET_MASK) - 1;
            let len = (inner >> Self::OFFSET_BITS) as usize;
            ExpandedSpanId::Inline(offset, len)
        } else {
            ExpandedSpanId::Interned((inner & !(1 << 63)) as usize)
        }
    }
}

impl std::fmt::Debug for SpanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.expand() {
            ExpandedSpanId::Inline(offset, len) => f
                .debug_struct("Inline")
                .field("offset", &offset)
                .field("len", &len)
                .finish(),
            ExpandedSpanId::Interned(i) => f.debug_tuple("Interned").field(&i).finish(),
        }
    }
}

enum ExpandedSpanId {
    Inline(u64, usize),
    Interned(usize),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpanContextId(usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceId(usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpanContext {
    Source(SourceId),
}

pub struct SpanManager {
    contexts: Vec<(u64, SpanContext)>,
    sources: Vec<SpanContextId>,
    // span interner
    span_to_idx: HashMap<(SpanContextId, usize, usize), usize>,
    idx_to_span: Vec<(SpanContextId, usize, usize)>,
}

impl Default for SpanManager {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl SpanManager {
    pub fn new() -> Self {
        Self {
            contexts: Vec::new(),
            sources: Vec::new(),
            span_to_idx: HashMap::default(),
            idx_to_span: Vec::new(),
        }
    }

    pub fn insert_source_context(&mut self, len: usize) -> (SpanContextId, SourceId) {
        let source_id = SourceId(self.sources.len());
        let context_id = self.insert_context(len, SpanContext::Source(source_id));
        self.sources.push(context_id);
        (context_id, source_id)
    }

    fn insert_context(&mut self, len: usize, context: SpanContext) -> SpanContextId {
        let len = u64::try_from(len).unwrap();
        let base_offset = if let Some(&(last_end, _)) = self.contexts.last() {
            last_end
        } else {
            0
        };
        let context_id = SpanContextId(self.contexts.len());
        self.contexts.push((base_offset + len + 1, context));
        context_id
    }

    #[must_use]
    pub fn get_context(&self, context: SpanContextId) -> &SpanContext {
        &self.contexts[context.0].1
    }

    #[must_use]
    fn get_context_offsets(&self, context: SpanContextId) -> (u64, u64) {
        let i = context.0;
        if i == 0 {
            (0, self.contexts[0].0)
        } else {
            (self.contexts[i - 1].0, self.contexts[i].0)
        }
    }

    #[must_use]
    fn get_context_from_offset(&self, offset: u64) -> SpanContextId {
        match self.contexts.binary_search_by_key(&offset, |entry| entry.0) {
            Ok(i) => SpanContextId(i + 1),
            Err(i) => SpanContextId(i),
        }
    }

    pub fn intern_span(&mut self, context: SpanContextId, start: usize, end: usize) -> SpanId {
        let start_u64 = u64::try_from(start).unwrap();
        let end_u64 = u64::try_from(end).unwrap();
        let (min_offset, max_offset) = self.get_context_offsets(context);
        assert!(start_u64 <= end_u64);
        let start_offset = min_offset + start_u64;
        let end_offset = min_offset + end_u64;
        assert!(start_offset < max_offset);
        assert!(end_offset < max_offset);

        let len = end_u64 - start_u64;
        if len > SpanId::LEN_MAX || start_offset >= SpanId::OFFSET_MASK {
            let span = (context, start, end);
            let i = match self.span_to_idx.entry(span) {
                std::collections::hash_map::Entry::Occupied(entry) => *entry.get(),
                std::collections::hash_map::Entry::Vacant(entry) => {
                    let i = self.idx_to_span.len();
                    self.idx_to_span.push(span);
                    entry.insert(i);
                    i
                }
            };
            SpanId(NonZeroU64::new((i as u64) | (1 << 63)).unwrap())
        } else {
            SpanId(NonZeroU64::new((start_offset + 1) | (len << SpanId::OFFSET_BITS)).unwrap())
        }
    }

    #[must_use]
    pub fn get_span(&self, span: SpanId) -> (SpanContextId, usize, usize) {
        match span.expand() {
            ExpandedSpanId::Inline(start_offset, len) => {
                let context = self.get_context_from_offset(start_offset);
                let (min_offset, _) = self.get_context_offsets(context);
                let start = (start_offset - min_offset) as usize;
                (context, start, start + len)
            }
            ExpandedSpanId::Interned(i) => self.idx_to_span[i],
        }
    }

    pub(crate) fn make_surrounding_span(&mut self, start_span: SpanId, end_span: SpanId) -> SpanId {
        let (start_ctx_id, start_start_pos, _) = self.get_span(start_span);
        let (end_ctx_id, _, end_end_pos) = self.get_span(end_span);
        assert_eq!(start_ctx_id, end_ctx_id);
        assert!(start_start_pos <= end_end_pos);
        self.intern_span(start_ctx_id, start_start_pos, end_end_pos)
    }
}
