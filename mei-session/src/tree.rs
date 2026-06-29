use serde::{Deserialize, Serialize};

use crate::entry::Entry;
use crate::error::SessionError;
use crate::ids::{NodeId, SessionId};

/// A node in a tree session: an entry plus a pointer to its parent.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    parent: Option<NodeId>,
    entry: Entry,
}

impl Node {
    /// The node's parent. `None` only at the root.
    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    pub fn entry(&self) -> &Entry {
        &self.entry
    }
}

/// Tree session: append-only, never removes. Diverging branches — the old branch
/// stays alive, off the active path.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TreeSession {
    id: SessionId,
    nodes: Vec<Node>,
    active: NodeId,
}

impl TreeSession {
    /// Creates the session with the root entry. A tree always starts with one
    /// node, so `active` is never null.
    pub fn new(id: SessionId, first: Entry) -> Self {
        Self {
            id,
            nodes: vec![Node {
                parent: None,
                entry: first,
            }],
            active: NodeId::new(0),
        }
    }

    pub fn id(&self) -> &SessionId {
        &self.id
    }

    /// The active node — where the next `push` branches from.
    pub fn active(&self) -> NodeId {
        self.active
    }

    /// The id of the root node.
    pub fn root_id(&self) -> NodeId {
        NodeId::new(0)
    }

    /// Appends an entry as a child of the active node and moves the cursor to it.
    /// If the active node already had a child (because you went back with
    /// `set_active`), this creates a new branch — the old one stays intact.
    pub fn push(&mut self, entry: Entry) -> NodeId {
        let id = NodeId::new(self.nodes.len() as u32);
        self.nodes.push(Node {
            parent: Some(self.active),
            entry,
        });
        self.active = id;
        id
    }

    /// Moves the cursor to an existing node. Going back and then `push` branches.
    pub fn set_active(&mut self, node: NodeId) -> Result<(), SessionError> {
        if (node.index() as usize) >= self.nodes.len() {
            return Err(SessionError::UnknownNode(node));
        }
        self.active = node;
        Ok(())
    }

    /// Everything the user sees: all nodes, with the branches. The structure
    /// comes from the `parent` pointers.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// What the model sees: the path from the root to the active node, cut at the
    /// last compaction.
    pub fn model_context(&self) -> Vec<&Entry> {
        let mut path: Vec<&Entry> = Vec::new();
        let mut current = Some(self.active);
        while let Some(id) = current {
            let node = &self.nodes[id.index() as usize];
            path.push(&node.entry);
            if node.entry.is_compaction() {
                break;
            }
            current = node.parent;
        }
        path.reverse(); // was active->root; becomes root->active.
        path
    }
}
