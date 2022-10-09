use std::io::Read;
use std::sync::Arc;

use bytes::{Buf, Bytes};
use h3::server::RequestStream;
use h3_quinn::BidiStream;
use http::header::CONTENT_LENGTH;
use http::{HeaderMap, Request, Response};
use hyper::service::Service;
use quinn::Connecting;
use tracing::info;

use crate::convert::HttpAdapter;
use crate::h3::BodyAdapter;
use crate::service::call_service;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP semantics error: {0}")]
    Http(#[from] http::Error),

    #[error("Invalid HTTP header found: {0}")]
    HttpHeader(#[from] http::header::InvalidHeaderValue),

    #[error("HTTP/3 error: {0}")]
    H3(#[from] h3::Error),

    #[error("QUIC connection error: {0}")]
    Quic(#[from] quinn::ConnectionError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Type conversion error: {0}")]
    Conversion(#[from] crate::convert::Error),

    #[error("Service error: {0}")]
    Service(Box<dyn std::error::Error + Send>),
}

pub struct Connection {
    inner: h3::server::Connection<h3_quinn::Connection, Bytes>,
}

impl Connection {
    pub async fn new(connecting: Connecting) -> Result<Self, Error> {
        let connection = connecting.await?;
        let id = connection.connection.stable_id();
        let inner = h3::server::Connection::new(h3_quinn::Connection::new(connection)).await?;

        info!("HTTP/3 connection initiated from connection ID {}", id);

        Ok(Self { inner })
    }

    pub async fn begin<S, E>(mut self, service: &Arc<S>) -> Result<(), Error>
    where
        S: Service<Request<Bytes>, Response = Response<Bytes>, Error = E>,
        S: Send + Sync + Clone + 'static,
        S::Future: Send,
        E: std::error::Error + Send + 'static,
    {
        loop {
            let (request, stream) = match self.inner.accept().await? {
                Some(v) => v,
                None => return Ok(()),
            };

            info!(
                "Incoming request accepted: {} {}",
                request.method(),
                request.uri()
            );

            tokio::spawn(Self::handle(request, stream, Arc::clone(service)));
        }
    }

    async fn handle<S, E>(
        request: Request<()>,
        mut stream: RequestStream<BidiStream<Bytes>, Bytes>,
        service: Arc<S>,
    ) -> Result<(), Error>
    where
        S: Service<Request<Bytes>, Response = Response<Bytes>, Error = E> + Send + Sync + Clone,
        S::Future: Send,
        E: std::error::Error + Send + 'static,
    {
        let content_length = request
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|v| {
                (|| -> Result<usize, Box<dyn std::error::Error>> { Ok(v.to_str()?.parse()?) })()
                    .ok()
            })
            .unwrap_or(0);

        let mut buffer = Vec::with_capacity(content_length);
        if let Some(data) = stream.recv_data().await? {
            data.reader().read_to_end(&mut buffer)?;
        }

        let adapter = BodyAdapter::new(Bytes::from(buffer));
        let response = call_service(&service, adapter.u_to_v(request).await?)
            .await
            .map_err(|e| Error::Service(Box::new(e)))?;

        stream
            .send_response(adapter.v_to_u(response).await?)
            .await?;

        stream.send_data(adapter.into_inner()?).await?;
        stream.send_trailers(HeaderMap::new()).await?;
        stream.finish().await?;

        Ok(())
    }
}
