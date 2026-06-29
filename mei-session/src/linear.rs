use serde::{Deserialize, Serialize};

use crate::entry::Entry;
use crate::error::SessionError;
use crate::ids::SessionId;

/// Linear session: a single, editable path. Diverging truncates the old tail.
///
/// `entries[..cursor]` is the current path; `entries[cursor..]` is the redo
/// buffer, discarded on the next `push`.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct LinearSession {
    id: SessionId,
    entries: Vec<Entry>,
    cursor: usize,
}

impl LinearSession {
    pub fn new(id: SessionId) -> Self {
        Self {
            id,
            entries: Vec::new(),
            cursor: 0,
        }
    }

    pub fn id(&self) -> &SessionId {
        &self.id
    }

    /// Appends an entry at the end of the current path. Any redo buffer left by
    /// an `undo` is discarded.
    pub fn push(&mut self, entry: Entry) {
        self.entries.truncate(self.cursor);
        self.entries.push(entry);
        self.cursor = self.entries.len();
    }

    /// Steps back one. `false` if already at the start.
    pub fn undo(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        true
    }

    /// Redoes one undone step. `false` if there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        if self.cursor >= self.entries.len() {
            return false;
        }
        self.cursor += 1;
        true
    }

    /// Everything the user sees: the whole current path.
    pub fn entries(&self) -> &[Entry] {
        &self.entries[..self.cursor]
    }

    /// What the model sees: from the last compaction to the end of the current path.
    pub fn model_context(&self) -> Vec<&Entry> {
        let active = &self.entries[..self.cursor];
        // The model context starts at the last compaction; with none, at the beginning.
        let start = active.iter().rposition(Entry::is_compaction).unwrap_or(0);
        active[start..].iter().collect()
    }

    pub(crate) fn validate(&self) -> Result<(), SessionError> {
        if self.cursor > self.entries.len() {
            return Err(SessionError::Corrupt("cursor out of range"));
        }
        Ok(())
    }
}
