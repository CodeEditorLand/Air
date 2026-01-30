mod common;

use common::mock_services::MockUpdateManager;
use Air::Updates::UpdateCheckRequest;

#[tokio::test]
async fn mock_update_manager_check_and_apply() {
    let mgr = MockUpdateManager::new();

    // Initially no updates
    let req = UpdateCheckRequest {};
    let res = mgr.check_for_updates(req).await;
    assert!(res.is_ok());
    let resp = res.unwrap();
    assert!(!resp.available);

    // Simulate available update by injecting via internal state
    {
        let mut updates = mgr.available_updates.lock().await;
        updates.push("1.2.3".to_string());
    }

    let res2 = mgr.check_for_updates(UpdateCheckRequest {}).await;
    assert!(res2.is_ok());
    let resp2 = res2.unwrap();
    assert!(resp2.available);

    let applied = mgr.apply_update("1.2.3").await;
    assert!(applied.is_ok());
    assert!(applied.unwrap());
}
