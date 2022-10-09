use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use futures::future::try_join;
use http::{Request, Response};
use hyper::service::Service;
use rustls::ServerConfig;

use crate::{h12, h3};

#[derive(Debug, thiserror::Error)]
pub enum JoinError {
    #[error(transparent)]
    H12(#[from] h12::Error),

    #[error(transparent)]
    H3(#[from] h3::Error),
}

pub struct Server<S, E> {
    h12: h12::Endpoint<S, E>,
    h3: h3::Endpoint<S, E>,
}

impl<S, E> Server<S, E> {
    pub fn new<A>(config: &ServerConfig, bind_to: A, service: Arc<S>) -> Self
    where
        A: Into<SocketAddr> + Copy,
    {
        Self {
            h12: h12::Endpoint::new(config, bind_to, Arc::clone(&service)),
            h3: h3::Endpoint::new(config, bind_to, Arc::clone(&service)),
        }
    }
}

impl<S, E> Server<S, E>
where
    S: Service<Request<Bytes>, Response = Response<Bytes>, Error = E>,
    S: Send + Sync + Clone + 'static,
    S::Future: Send,
    E: std::error::Error + Send + 'static,
{
    pub async fn begin(self) -> Result<(), JoinError> {
        try_join(
            async move { self.h12.begin().await.map_err(JoinError::from) },
            async move { self.h3.begin().await.map_err(JoinError::from) },
        )
        .await
        .map(|_| ())
    }
}
