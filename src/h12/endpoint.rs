use std::convert::Infallible;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use hyper::server::conn::AddrIncoming;
use hyper::service::{make_service_fn, service_fn, Service};
use hyper::{Body, Server};
use tracing::{error, info};

use crate::convert::HttpAdapter;
use crate::h12::tls::{TlsAcceptor, TlsStream};
use crate::h12::BodyAdapter;
use crate::service::call_service;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP semantics error: {0}")]
    Http(#[from] http::Error),

    #[error("Hyper error: {0}")]
    Hyper(#[from] hyper::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Type conversion error: {0}")]
    Conversion(#[from] crate::convert::Error),

    #[error("Service error: {0}")]
    Service(Box<dyn std::error::Error>),
}

pub struct Endpoint<S, E> {
    rustls_config: rustls::ServerConfig,
    bind_to: SocketAddr,
    service: Arc<S>,
    _phantom: PhantomData<fn() -> E>,
}

impl<S, E> Endpoint<S, E> {
    pub fn new<A>(rustls_config: &rustls::ServerConfig, bind_to: A, service: Arc<S>) -> Self
    where
        A: Into<SocketAddr>,
    {
        let mut rustls_config = rustls_config.clone();

        rustls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        Self {
            rustls_config,
            bind_to: bind_to.into(),
            service,
            _phantom: PhantomData,
        }
    }
}

impl<S, E> Endpoint<S, E>
where
    S: Service<Request<Bytes>, Response = Response<Bytes>, Error = E>,
    S: Send + Sync + Clone + 'static,
    S::Future: Send,
    E: std::error::Error + 'static,
{
    pub async fn begin(self) -> Result<(), Error> {
        let make_service = make_service_fn(|stream: &TlsStream| {
            info!(
                "Connecting from {}",
                stream
                    .remote_addr()
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "<unknown>".to_owned()),
            );

            let service = self.service.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |request: Request<Body>| {
                    let service = service.clone();
                    async move {
                        match Self::adapter(request, &service).await {
                            Ok(r) => Ok::<_, Infallible>(r),
                            Err(e) => {
                                error!("{}", e);

                                Ok(Response::builder()
                                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                                    .body(Body::empty())
                                    .unwrap())
                            }
                        }
                    }
                }))
            }
        });

        let server = Server::builder(TlsAcceptor::new(
            Arc::new(self.rustls_config),
            AddrIncoming::bind(&self.bind_to)?,
        ))
        .serve(make_service);

        info!("HTTP/1.1 and HTTP/2 endpoint started at: {}", &self.bind_to);

        Ok(server.await?)
    }

    async fn adapter(request: Request<Body>, service: &Arc<S>) -> Result<Response<Body>, Error> {
        let adapter = BodyAdapter;
        let response = call_service(service, adapter.u_to_v(request).await?)
            .await
            .map_err(|e| Error::Service(Box::new(e)))?;

        Ok(adapter.v_to_u(response).await?)
    }
}
