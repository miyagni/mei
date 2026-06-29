use serde::{Deserialize, Serialize};

/// Stable id of a session. Becomes the file name on disk.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Id of a node in a tree session.
///
/// Today it is the index into the append-only log. Since the tree never removes
/// a node, the index is stable.
// TODO: if gc/compaction that removes nodes is ever added, switch to an explicit id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeId(u32);

impl NodeId {
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    pub fn index(self) -> u32 {
        self.0
    }
}
