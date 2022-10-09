use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use http::header::CONTENT_TYPE;
use http::{Request, Response, StatusCode};
use hyper::service::Service;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;
use tracing::info;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP semantics error: {0}")]
    Http(#[from] http::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone)]
pub struct StaticFileService {
    root: PathBuf,
}

impl StaticFileService {
    pub fn new<P>(root: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    fn real_path_of<P>(&self, path: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.root
            .components()
            .chain(
                path.as_ref()
                    .components()
                    .filter(|c| !matches!(c, Component::ParentDir | Component::RootDir)),
            )
            .collect::<PathBuf>()
    }
}

impl Service<Request<Bytes>> for StaticFileService {
    type Response = Response<Bytes>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Bytes>) -> Self::Future {
        let path = PathBuf::from(req.uri().path());
        let path = self.real_path_of(&path);

        info!("Real path is {}", path.to_str().unwrap());

        Box::pin(async move {
            if !path.exists() || !path.is_file() {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Bytes::new())?);
            }

            let content_type = mime_guess::from_path(path.clone()).first_or_octet_stream();
            let file = File::open(path).await?;
            let mut buffer = Vec::<u8>::with_capacity(file.metadata().await?.len() as usize);
            let mut reader = BufReader::new(file);

            reader.read_to_end(&mut buffer).await?;

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, content_type.as_ref())
                .body(Bytes::from(buffer))?)
        })
    }
}

pub async fn call_service<S, E, Req, Res>(service: &Arc<S>, request: Req) -> Result<Res, E>
where
    S: Service<Req, Response = Res, Error = E> + Clone,
{
    service.as_ref().clone().call(request).await
}
