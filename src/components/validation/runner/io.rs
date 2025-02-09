use std::convert::Infallible;

use http::{Request, Response};
use hyper::Body;
use tokio::{pin, select, sync::mpsc};
use tonic::{
    body::BoxBody,
    transport::{Channel, Endpoint, NamedService},
    Status,
};
use tower::Service;
use vector_common::shutdown::ShutdownSignal;
use vector_core::{event::Event, tls::MaybeTlsSettings};

use crate::{
    components::validation::{
        sync::{Configuring, TaskCoordinator},
        util::GrpcAddress,
        TestEvent,
    },
    proto::vector::{
        Client as VectorClient, HealthCheckRequest, HealthCheckResponse, PushEventsRequest,
        PushEventsResponse, Server as VectorServer, Service as VectorService, ServingStatus,
    },
    sources::util::grpc::run_grpc_server,
};

#[derive(Clone)]
pub struct EventForwardService {
    tx: mpsc::Sender<Event>,
}

impl From<mpsc::Sender<Event>> for EventForwardService {
    fn from(tx: mpsc::Sender<Event>) -> Self {
        Self { tx }
    }
}

#[tonic::async_trait]
impl VectorService for EventForwardService {
    async fn push_events(
        &self,
        request: tonic::Request<PushEventsRequest>,
    ) -> Result<tonic::Response<PushEventsResponse>, Status> {
        let events = request.into_inner().events.into_iter().map(Event::from);

        for event in events {
            self.tx
                .send(event)
                .await
                .expect("event forward rx should not close first");
        }

        Ok(tonic::Response::new(PushEventsResponse {}))
    }

    async fn health_check(
        &self,
        _: tonic::Request<HealthCheckRequest>,
    ) -> Result<tonic::Response<HealthCheckResponse>, Status> {
        let message = HealthCheckResponse {
            status: ServingStatus::Serving.into(),
        };

        Ok(tonic::Response::new(message))
    }
}

pub struct InputEdge {
    #[allow(dead_code)]
    client: VectorClient<Channel>,
}

pub struct OutputEdge {
    listen_addr: GrpcAddress,
    service: VectorServer<EventForwardService>,
    rx: mpsc::Receiver<Event>,
}

impl InputEdge {
    pub fn from_address(address: GrpcAddress) -> Self {
        let channel = Endpoint::from(address.as_uri()).connect_lazy();
        Self {
            client: VectorClient::new(channel),
        }
    }

    pub fn spawn_input_client(
        self,
        task_coordinator: &TaskCoordinator<Configuring>,
    ) -> mpsc::Sender<TestEvent> {
        let (tx, mut rx) = mpsc::channel(1024);
        let started = task_coordinator.track_started();
        let completed = task_coordinator.track_completed();

        tokio::spawn(async move {
            started.mark_as_done();

            // TODO: Read events from `rx` and send them to the component topology via our Vector
            // gRPC client that connects to the Vector source.
            while let Some(_event) = rx.recv().await {}

            completed.mark_as_done();
        });

        tx
    }
}

impl OutputEdge {
    pub fn from_address(listen_addr: GrpcAddress) -> Self {
        let (tx, rx) = mpsc::channel(1024);

        Self {
            listen_addr,
            service: VectorServer::new(EventForwardService::from(tx)),
            rx,
        }
    }

    pub fn spawn_output_server(
        self,
        task_coordinator: &TaskCoordinator<Configuring>,
    ) -> mpsc::Receiver<Event> {
        spawn_grpc_server(self.listen_addr, self.service, task_coordinator);
        self.rx
    }
}

pub fn spawn_grpc_server<S>(
    listen_addr: GrpcAddress,
    service: S,
    task_coordinator: &TaskCoordinator<Configuring>,
) where
    S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible>
        + NamedService
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    let started = task_coordinator.track_started();
    let completed = task_coordinator.track_completed();
    let mut shutdown_handle = task_coordinator.register_for_shutdown();

    tokio::spawn(async move {
        started.mark_as_done();

        let (trigger_shutdown, shutdown_signal, _) = ShutdownSignal::new_wired();
        let mut trigger_shutdown = Some(trigger_shutdown);
        let tls_settings = MaybeTlsSettings::from_config(&None, true)
            .expect("should not fail to get empty TLS settings");

        let server = run_grpc_server(
            listen_addr.as_socket_addr(),
            tls_settings,
            service,
            shutdown_signal,
        );
        pin!(server);

        loop {
            select! {
                // Propagate our shutdown signal to the shutdown signal that `run_grpc_server` needs.
                _ = shutdown_handle.wait(), if trigger_shutdown.is_some() => {
                    trigger_shutdown.take().unwrap().cancel();
                },
                // TODO: Should we check the return value here to see if its an `Err`?
                _ = &mut server => break,
            }
        }

        completed.mark_as_done();
    });
}

pub struct ControlledEdges {
    pub input: Option<mpsc::Sender<TestEvent>>,
    pub output: Option<mpsc::Receiver<Event>>,
}
