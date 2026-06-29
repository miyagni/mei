use mei_session::{Entry, LinearSession, SessionId};

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
