use mei_provider::{AuthStore, Credential, OAuthToken};

#[test]
fn credential_round_trips_via_json() {
    let cases = [
        Credential::ApiKey("sk-test".into()),
        Credential::OAuth(OAuthToken {
            access_token: "at".into(),
            refresh_token: Some("rt".into()),
            expires_at: Some(1_234_567_890),
        }),
    ];

    for cred in cases {
        let json = serde_json::to_string(&cred).expect("serializes");
        let back: Credential = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(cred, back);
    }
}

#[test]
fn set_get_remove_persists_across_reopen() {
    let dir = tempfile::tempdir().expect("tempdir");

    {
        let mut store = AuthStore::open_in(dir.path()).expect("open");
        store
            .set("anthropic", Credential::ApiKey("sk-ant".into()))
            .expect("set anthropic");
        store
            .set("openai", Credential::ApiKey("sk-oai".into()))
            .expect("set openai");
    }

    // Reopened from disk: both credentials survived.
    let store = AuthStore::open_in(dir.path()).expect("reopen");
    assert_eq!(
        store.get("anthropic"),
        Some(&Credential::ApiKey("sk-ant".into()))
    );
    let mut listed: Vec<&str> = store.providers().collect();
    listed.sort_unstable();
    assert_eq!(listed, ["anthropic", "openai"]);

    // Remove one, persist.
    let mut store = store;
    assert!(store.remove("openai").expect("remove"));
    assert!(!store.remove("openai").expect("remove again")); // already gone

    // Reopened: openai is gone, anthropic stays.
    let store = AuthStore::open_in(dir.path()).expect("reopen");
    assert_eq!(store.get("openai"), None);
    assert_eq!(
        store.get("anthropic"),
        Some(&Credential::ApiKey("sk-ant".into()))
    );
}

#[test]
fn open_missing_dir_is_empty_store() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = AuthStore::open_in(dir.path().join("not-created-yet")).expect("open empty");
    assert!(store.get("anthropic").is_none());
    assert_eq!(store.providers().count(), 0);
}
