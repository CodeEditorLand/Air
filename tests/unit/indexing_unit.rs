mod common;

use common::mock_services::MockFileIndexer;
use Air::Indexing::IndexRequest;

#[tokio::test]
async fn mock_indexer_index_and_search() {
    let idx = MockFileIndexer::new();

    let req = IndexRequest { path: "src/lib".to_string() };
    let res = idx.index_files(req).await;
    assert!(res.is_ok());
    let resp = res.unwrap();
    assert!(resp.success);

    let search = idx.search_files("lib").await;
    assert!(search.is_ok());
    let files = search.unwrap();
    assert!(files.len() >= 1);
}
