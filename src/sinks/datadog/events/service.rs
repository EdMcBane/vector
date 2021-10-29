use crate::sinks::util::http::HttpBatchService;

use crate::event::EventStatus;

use http::Request;

use crate::http::HttpClient;
use crate::sinks::datadog::events::request_builder::DatadogEventsRequest;
use crate::sinks::util::sink::Response;
use futures::future;
use futures::future::BoxFuture;
use futures::future::Ready;
use hyper::Body;
use std::task::{Context, Poll};
use tower::{Service, ServiceExt};
use vector_core::internal_event::EventsSent;
use vector_core::stream::DriverResponse;

pub struct DatadogEventsResponse {
    pub event_status: EventStatus,
    pub http_status: http::StatusCode,
    pub event_byte_size: usize,
}

impl DriverResponse for DatadogEventsResponse {
    fn event_status(&self) -> EventStatus {
        self.event_status
    }

    fn events_sent(&self) -> EventsSent {
        EventsSent {
            count: 1,
            byte_size: self.event_byte_size,
        }
    }
}

#[derive(Clone)]
pub struct DatadogEventsService {
    batch_http_service:
        HttpBatchService<Ready<Result<http::Request<Vec<u8>>, crate::Error>>, DatadogEventsRequest>,
}

impl DatadogEventsService {
    pub fn new(endpoint: String, default_api_key: String, http_client: HttpClient<Body>) -> Self {
        let batch_http_service = HttpBatchService::new(http_client, move |req| {
            let req: DatadogEventsRequest = req;

            let api_key = match req.metadata.api_key.as_ref() {
                Some(x) => x.as_ref(),
                None => default_api_key.as_str(),
            };

            let request = Request::post(endpoint.as_str())
                .header("Content-Type", "application/json")
                .header("DD-API-KEY", api_key)
                .header("Content-Length", req.body.len())
                .body(req.body)
                .map_err(|x| x.into());
            future::ready(request)
        });
        Self { batch_http_service }
    }
}

impl Service<DatadogEventsRequest> for DatadogEventsService {
    type Response = DatadogEventsResponse;
    type Error = crate::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: DatadogEventsRequest) -> Self::Future {
        let mut http_service = self.batch_http_service.clone();

        Box::pin(async move {
            http_service.ready().await?;
            let event_byte_size = req.metadata.event_byte_size;
            let http_response = http_service.call(req).await?;
            let event_status = if http_response.is_successful() {
                EventStatus::Delivered
            } else if http_response.is_transient() {
                EventStatus::Errored
            } else {
                EventStatus::Failed
            };
            Ok(DatadogEventsResponse {
                event_status,
                http_status: http_response.status(),
                event_byte_size,
            })
        })
    }
}