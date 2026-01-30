pub mod mock_services {
    // Reuse the root mock_services.rs for unit tests via include!
    include!("../mock_services.rs");
}

pub mod helpers {
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};

    use crate::mock_services::*;

    // Simple helper to await short duration in async tests
    pub async fn short_delay() {
        sleep(Duration::from_millis(10)).await;
    }
}
