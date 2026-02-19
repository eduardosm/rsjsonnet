use std::cell::OnceCell;
use std::collections::HashMap;

/// A storage of informacion about source files.
///
/// It contains the information required by functions of the
/// [`report`](crate::report) module to render annotations.
pub(crate) struct SrcManager {
    files: HashMap<rsjsonnet_lang::span::SourceId, File>,
}

struct File {
    repr_path: String,
    data: Box<[u8]>,
    snippet: OnceCell<sourceannot::Snippet>,
}

impl Default for SrcManager {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl SrcManager {
    pub(crate) fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub(crate) fn insert_file(
        &mut self,
        id: rsjsonnet_lang::span::SourceId,
        repr_path: String,
        data: Box<[u8]>,
    ) {
        self.files.insert(
            id,
            File {
                repr_path,
                data,
                snippet: OnceCell::new(),
            },
        );
    }

    #[must_use]
    pub(crate) fn get_file_data(&self, id: rsjsonnet_lang::span::SourceId) -> &[u8] {
        &self.files[&id].data
    }

    #[must_use]
    pub(crate) fn get_file_repr_path(&self, id: rsjsonnet_lang::span::SourceId) -> &str {
        &self.files[&id].repr_path
    }

    #[must_use]
    pub(crate) fn get_file_snippet(
        &self,
        id: rsjsonnet_lang::span::SourceId,
    ) -> &sourceannot::Snippet {
        let file = &self.files[&id];
        file.snippet.get_or_init(|| {
            sourceannot::Snippet::with_utf8_bytes(
                1,
                &file.data,
                4,
                sourceannot::ControlCharStyle::Codepoint,
                true,
                sourceannot::InvalidSeqStyle::Hexadecimal,
                true,
            )
        })
    }
}
