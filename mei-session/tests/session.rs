use mei_session::{
    Entry, LinearSession, NodeId, Session, SessionError, SessionId, ToolCall, ToolResult,
    TreeSession,
};

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

#[test]
fn linear_round_trip_via_json() {
    let mut s = LinearSession::new(SessionId::new("s1"));
    s.push(Entry::user("hi"));
    s.push(Entry::assistant("hello"));

    let session = Session::Linear(s);
    let json = session.to_json().expect("serializes");
    let back = Session::from_json(&json).expect("deserializes");

    assert_eq!(session, back);
}

#[test]
fn tree_round_trip_via_json() {
    let mut t = TreeSession::new(SessionId::new("s1"), Entry::user("a"));
    t.push(Entry::assistant("b"));

    let session = Session::Tree(t);
    let json = session.to_json().expect("serializes");
    let back = Session::from_json(&json).expect("deserializes");

    assert_eq!(session, back);
}

#[test]
fn from_json_rejects_corrupt_tree() {
    // active points to a node that does not exist.
    let json = r#"{"kind":"tree","id":"s1","nodes":[{"parent":null,"entry":{"role":"user","content":"a"}}],"active":99}"#;
    assert!(Session::from_json(json).is_err());
}

#[test]
fn session_model_context_reads_without_matching_kind() {
    let mut t = TreeSession::new(SessionId::new("s1"), Entry::user("hi"));
    t.push(Entry::assistant("hello"));
    let session = Session::Tree(t);

    // read id and model context off the enum directly, no match on Linear/Tree.
    assert_eq!(session.id().as_str(), "s1");
    assert_eq!(
        session.model_context(),
        vec![&Entry::user("hi"), &Entry::assistant("hello")]
    );
}

#[test]
fn round_trip_preserves_tool_call_and_result() {
    // locks the canonical (provider-neutral) tool format across a round-trip.
    let mut s = LinearSession::new(SessionId::new("s1"));
    s.push(Entry::user("run the tool"));
    s.push(Entry::ToolCall(ToolCall {
        id: "call_1".into(),
        name: "read_file".into(),
        arguments: serde_json::json!({ "path": "src/lib.rs" }),
    }));
    s.push(Entry::ToolResult(ToolResult {
        call_id: "call_1".into(),
        output: "content".into(),
    }));

    let session = Session::Linear(s);
    let json = session.to_json().expect("serializes");
    let back = Session::from_json(&json).expect("deserializes");

    assert_eq!(session, back);
}

#[test]
fn save_then_load_round_trips_through_a_real_file() {
    let dir = tempfile::tempdir().expect("tempdir"); // removed (with its files) on drop
    let path = dir.path().join("s1.json");

    let mut s = LinearSession::new(SessionId::new("s1"));
    s.push(Entry::user("hi"));
    s.push(Entry::assistant("hello"));
    let session = Session::Linear(s);

    session.save(&path).expect("save");
    assert!(path.exists()); // save really hit the disk
    let back = Session::load(&path).expect("load");

    assert_eq!(session, back);
}

#[test]
fn load_missing_file_errors() {
    // The fs path is reached, finds nothing, and writes nothing to disk.
    assert!(Session::load("this/path/does/not/exist.json").is_err());
}

#[test]
fn save_atomically_replaces_an_existing_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("s.json");

    let mut a = LinearSession::new(SessionId::new("s"));
    a.push(Entry::user("first"));
    Session::Linear(a).save(&path).expect("save a");

    let mut b = LinearSession::new(SessionId::new("s"));
    b.push(Entry::user("second"));
    let session_b = Session::Linear(b);
    session_b
        .save(&path)
        .expect("save b over the existing file");

    assert_eq!(Session::load(&path).expect("load"), session_b);
}
