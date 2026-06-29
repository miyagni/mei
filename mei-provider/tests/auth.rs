use mei_provider::{Credential, OAuthToken};

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
