#![forbid(future_incompatible, unsafe_code)]

//! Tide middleware for [`prometheus`] with a few default metrics.
//!
//! ## Example
//!
//! ```rust
//! # async_std::task::block_on(async {
//! let mut server = tide::new();
//!
//! server.with(tide_prometheus::Prometheus::new("tide"));
//!
//! // Optionally serve these metrics on the same server:
//! server.at("/metrics").get(tide_prometheus::metrics_endpoint);
//! # });
//! ```
//!
//! ## Metrics
//!
//! The `{prefix}` below is the string you put in [`Prometheus::new`].
//!
//! * `{prefix}_http_requests` ([`prometheus::IntCounterVec`]) with labels:
//!   * `method` as the request method.
//!   * `status` as the response status.
//!
//! ## Features
//!
//! * `process` will enable the [`prometheus`] `process` feature, recording
//! various metrics of the process.

pub use prometheus;

use prometheus::Encoder;

/// Tide middleware for [`prometheus`] with a few default metrics.
#[derive(Clone, Debug)]
pub struct Prometheus {
  /// The `{prefix}_http_requests` counter.
  http_requests: prometheus::IntCounterVec,
}

impl Prometheus {
  /// Creates a new Prometheus middleware. This also creates and registers the
  /// metrics in the [`prometheus::default_registry`].
  pub fn new(prefix: &str) -> Self {
    Self {
      http_requests: Self::http_requests(prefix),
    }
  }

  /// Creates, registers and returns the `{prefix}_http_requests` counter.
  fn http_requests(prefix: &str) -> prometheus::IntCounterVec {
    let name = format!("{}_http_requests", prefix);
    let opts = prometheus::Opts::new(name, "Counts http requests");
    prometheus::register_int_counter_vec!(opts, &["method", "code"]).unwrap()
  }
}

#[tide::utils::async_trait]
impl<State: Clone + Send + Sync + 'static> tide::Middleware<State>
  for Prometheus
{
  async fn handle(
    &self,
    request: tide::Request<State>,
    next: tide::Next<'_, State>,
  ) -> tide::Result {
    let method = request.method();
    let response = next.run(request).await;
    let status_code = response.status().to_string();

    self
      .http_requests
      .with_label_values(&[method.as_ref(), &status_code])
      .inc();

    Ok(response)
  }
}

/// A convencience [`tide::Endpoint`] that gathers the metrics with
/// [`prometheus::gather`] and then returns them inside the response body as
/// specified by [the Prometheus docs](https://prometheus.io/docs/instrumenting/exposition_formats/#text-based-format).
///
/// ## Example
///
/// ```rust
/// # async_std::task::block_on(async {
/// let mut server = tide::new();
/// server.with(tide_prometheus::Prometheus::new("tide"));
/// server.at("/metrics").get(tide_prometheus::metrics_endpoint);
/// # });
/// ```
///
/// Note that serving the metrics on the same server they're counted on will
/// make Prometheus's scraping also get counted in them. If you want to avoid
/// this you can use something like
/// [`tide-fluent-routes`](https://crates.io/crates/tide-fluent-routes) and have
/// separate trees for your main routes and the one for the metrics endpoint. Or
/// you can just run a completely separate server in the same program.
pub async fn metrics_endpoint<State>(
  _request: tide::Request<State>,
) -> tide::Result {
  let encoder = prometheus::TextEncoder::new();
  let metric_families = prometheus::gather();

  let mut buffer = vec![];
  encoder.encode(&metric_families, &mut buffer)?;
  let metrics = String::from_utf8(buffer)?;

  Ok(
    tide::Response::builder(tide::StatusCode::Ok)
      .content_type(prometheus::TEXT_FORMAT)
      .body(metrics)
      .build(),
  )
}
