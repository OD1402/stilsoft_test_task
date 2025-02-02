
use super::*;
// use mockito::mock;
use reqwest::StatusCode;

#[tokio::test]
async fn test_download_success() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("GET", "/test_url")
        .with_status(StatusCode::OK.as_u16().into())
        .with_body("test!!\nThis is a test.")
        .create();

    let client = Arc::new(Client::new());
    let result =
        download_and_line_count(&client, &format!("{}{}", server.url(), "/test_url"), 5).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().line_count, 2);
}

#[tokio::test]
async fn test_download_404() {
    let opts = mockito::ServerOpts {
        host: "127.0.0.1",
        port: 1234,
        ..Default::default()
    };
    let mut server = mockito::Server::new_with_opts_async(opts).await;
    let _mock = server
        .mock("GET", "/test_url")
        .with_status(StatusCode::NOT_FOUND.as_u16().into())
        .with_body("test!!\nThis is a test.")
        .create();

    let client = Arc::new(Client::new());
    let result =
        download_and_line_count(&client, &format!("{}{}", server.url(), "/test_url"), 5).await;

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "Status code: '404 Not Found' for URL: http://127.0.0.1:1234/test_url\n".to_string()
    );
}
