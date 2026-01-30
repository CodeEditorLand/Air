mod common;

use common::mock_services::MockAuthenticationService;

#[tokio::test]
async fn mock_authentication_basic_flow() {
    let auth = MockAuthenticationService::new();

    // Authenticate a client and validate token
    let client_id = "client-123";
    let res = auth.authenticate(client_id, "password").await;
    assert!(res.is_ok());
    assert!(res.unwrap());

    let valid = auth.validate_token(client_id).await;
    assert!(valid.is_ok());
    assert!(valid.unwrap());
}
