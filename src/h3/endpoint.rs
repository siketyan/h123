use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use futures::StreamExt;
use h3::error::Code;
use http::{Request, Response};
use hyper::service::Service;
use quinn::ServerConfig;
use tracing::{error, info};

use crate::h3::connection::{Connection, Error as ConnectionError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP/3 error: {0}")]
    H3(#[from] h3::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct Endpoint<S, E> {
    config: ServerConfig,
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

        rustls_config.max_early_data_size = u32::MAX;
        rustls_config.alpn_protocols = vec![b"h3".to_vec()];

        Self {
            config: ServerConfig::with_crypto(Arc::new(rustls_config)),
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
    E: std::error::Error + Send + 'static,
{
    pub async fn begin(self) -> Result<(), Error> {
        let (endpoint, mut incoming) = quinn::Endpoint::server(self.config, self.bind_to)?;

        info!("HTTP/3 endpoint started at: {}", &self.bind_to);

        while let Some(connection) = incoming.next().await {
            info!("Connecting from {}", connection.remote_address());

            let service = Arc::clone(&self.service);
            tokio::spawn(async move {
                let connection = match Connection::new(connection).await {
                    Ok(c) => c,
                    Err(e) => {
                        error!("{}", e);

                        return;
                    }
                };

                match connection.begin(&service).await {
                    Ok(c) => c,
                    Err(e) => {
                        if let ConnectionError::H3(ref e) = e {
                            if e.try_get_code()
                                .map(|c| c == Code::H3_NO_ERROR || c == 0x0)
                                .unwrap_or(true)
                            {
                                info!("Connection closed with no error.");

                                return;
                            }
                        }

                        error!("{}", e);
                    }
                };
            });
        }

        endpoint.wait_idle().await;

        Ok(())
    }
}
