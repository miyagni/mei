use mei_session::Entry;

#[test]
fn compaction_entry_is_detected() {
    assert!(Entry::compaction("summary").is_compaction());
    assert!(!Entry::user("hi").is_compaction());
}
