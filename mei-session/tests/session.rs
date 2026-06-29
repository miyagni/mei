use mei_session::{Entry, LinearSession, NodeId, SessionError, SessionId, TreeSession};

#[test]
fn compaction_entry_is_detected() {
    assert!(Entry::compaction("summary").is_compaction());
    assert!(!Entry::user("hi").is_compaction());
}

#[test]
fn linear_undo_then_push_truncates_tail() {
    let mut s = LinearSession::new(SessionId::new("s1"));
    s.push(Entry::user("a"));
    s.push(Entry::assistant("b"));
    s.push(Entry::user("c"));

    assert!(s.undo()); // back to just after "b"
    s.push(Entry::user("c2")); // discards "c"

    let expected = [Entry::user("a"), Entry::assistant("b"), Entry::user("c2")];
    assert_eq!(s.entries(), expected.as_slice());
}

#[test]
fn linear_model_context_starts_at_last_compaction() {
    let mut s = LinearSession::new(SessionId::new("s1"));
    s.push(Entry::user("a"));
    s.push(Entry::assistant("b"));
    s.push(Entry::compaction("summary of a,b"));
    s.push(Entry::user("d"));

    let context = s.model_context();
    assert_eq!(
        context,
        vec![&Entry::compaction("summary of a,b"), &Entry::user("d")]
    );
    assert_eq!(s.entries().len(), 4);
}

#[test]
fn undo_redo_respect_bounds() {
    let mut s = LinearSession::new(SessionId::new("s1"));
    assert!(!s.undo()); // nothing to undo
    s.push(Entry::user("a"));
    assert!(s.undo()); // undoes "a"
    assert!(!s.undo()); // already at the start
    assert!(s.redo()); // redoes "a"
    assert!(!s.redo()); // nothing to redo
}

#[test]
fn tree_branch_keeps_old_path_out_of_context() {
    let mut t = TreeSession::new(SessionId::new("s1"), Entry::user("a"));
    t.push(Entry::assistant("b"));
    t.push(Entry::user("c")); // a -> b -> c

    // go back to the root and branch: a -> e -> f.
    t.set_active(t.root_id()).expect("root exists");
    t.push(Entry::assistant("e"));
    t.push(Entry::user("f"));

    // the model context is the active path: a, e, f.
    let context = t.model_context();
    assert_eq!(
        context,
        vec![&Entry::user("a"), &Entry::assistant("e"), &Entry::user("f")]
    );
    // but b and c still exist: nodes() sees all 5.
    assert_eq!(t.nodes().len(), 5);
}

#[test]
fn tree_model_context_starts_at_last_compaction() {
    let mut t = TreeSession::new(SessionId::new("s1"), Entry::user("a"));
    t.push(Entry::assistant("b"));
    t.push(Entry::compaction("summary of a,b"));
    t.push(Entry::user("d")); // a -> b -> compaction -> d

    let context = t.model_context();
    assert_eq!(
        context,
        vec![&Entry::compaction("summary of a,b"), &Entry::user("d")]
    );
    assert_eq!(t.nodes().len(), 4);
}

#[test]
fn set_active_to_unknown_node_errors() {
    let mut t = TreeSession::new(SessionId::new("s1"), Entry::user("a"));
    let err = t.set_active(NodeId::new(99)).unwrap_err();
    assert!(matches!(err, SessionError::UnknownNode(_)));
}
