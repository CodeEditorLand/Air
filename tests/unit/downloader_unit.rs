mod common;

use common::mock_services::MockDownloadManager;
use Air::Downloader::DownloadRequest;

#[tokio::test]
async fn mock_downloader_download_and_progress() {
    let mgr = MockDownloadManager::new();

    let req = DownloadRequest { url: "http://example.com/file.bin".to_string() };
    let res = mgr.download_file(req).await;
    assert!(res.is_ok());
    let resp = res.unwrap();
    assert!(resp.success);

    let progress = mgr.get_download_progress("http://example.com/file.bin").await;
    assert!(progress.is_ok());
}
