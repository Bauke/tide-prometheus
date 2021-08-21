use tide_testing::TideTestingExt;

const METRICS: &str = r#"
# HELP custom_http_requests Counts http requests
# TYPE custom_http_requests counter
custom_http_requests{code="500",method="DELETE"} 1
custom_http_requests{code="500",method="HEAD"} 1
# HELP tide_http_requests Counts http requests
# TYPE tide_http_requests counter
tide_http_requests{code="200",method="GET"} 1
tide_http_requests{code="200",method="POST"} 1
"#;

#[async_std::test]
async fn test_metrics() {
  let mut server = tide::new();
  server.at("/metrics").get(tide_prometheus::metrics_endpoint);

  let tide_prefix = tide_prometheus::Prometheus::new("tide");
  server.at("/ok").with(tide_prefix.clone()).all(ok_route);

  for request in vec![server.get("/ok"), server.post("/ok")] {
    let response = request.await.unwrap();
    assert_eq!(response.status(), tide::StatusCode::Ok)
  }

  let own_prefix = tide_prometheus::Prometheus::new("custom");
  server.at("/ise").with(own_prefix.clone()).all(ise_route);

  for request in vec![server.head("/ise"), server.delete("/ise")] {
    let response = request.await.unwrap();
    assert_eq!(response.status(), tide::StatusCode::InternalServerError)
  }

  let mut metrics_response = server.get("/metrics").await.unwrap();
  assert_eq!(
    metrics_response.content_type().unwrap(),
    tide::http::Mime::from(prometheus::TEXT_FORMAT)
  );
  let metrics = metrics_response.body_string().await.unwrap();

  for line in METRICS.trim().lines() {
    assert!(metrics.contains(line));
  }
}

async fn ok_route(_request: tide::Request<()>) -> tide::Result {
  Ok(tide::Response::new(tide::StatusCode::Ok))
}

async fn ise_route(_request: tide::Request<()>) -> tide::Result {
  Ok(tide::Response::new(tide::StatusCode::InternalServerError))
}
