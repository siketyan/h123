use std::fmt::{Display, Formatter};

use async_trait::async_trait;
use http::{Request, Response};

#[derive(Debug)]
pub struct Error {
    inner: Box<dyn std::error::Error + Send>,
}

impl Error {
    pub fn boxed(err: impl std::error::Error + Send + 'static) -> Self {
        Self {
            inner: Box::new(err),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

#[async_trait]
pub trait Adapter<U, V>: Sync {
    async fn u_to_v(&self, u: U) -> Result<V>;
    async fn v_to_u(&self, v: V) -> Result<U>;
}

pub trait HttpHeaderAdapter {
    fn response_header(&self, response: http::response::Builder) -> http::response::Builder {
        response
    }
}

#[async_trait]
pub trait HttpAdapter<U, V>: Sync {
    async fn u_to_v(&self, u: Request<U>) -> Result<Request<V>>
    where
        U: Send + 'async_trait;
    async fn v_to_u(&self, v: Response<V>) -> Result<Response<U>>
    where
        V: Send + 'async_trait;
}

#[async_trait]
impl<T, U, V> HttpAdapter<U, V> for T
where
    T: Adapter<U, V> + HttpHeaderAdapter,
{
    async fn u_to_v(&self, u: Request<U>) -> Result<Request<V>>
    where
        U: Send + 'async_trait,
    {
        let mut builder = Request::builder()
            .method(u.method().clone())
            .uri(u.uri().clone())
            .version(u.version());

        for (k, v) in u.headers() {
            builder = builder.header(k, v.clone());
        }

        builder
            .body(self.u_to_v(u.into_body()).await?)
            .map_err(Error::boxed)
    }

    async fn v_to_u(&self, v: Response<V>) -> Result<Response<U>>
    where
        V: Send + 'async_trait,
    {
        let mut builder = Response::builder().status(v.status()).version(v.version());

        for (k, v) in v.headers() {
            builder = builder.header(k, v.clone());
        }

        self.response_header(builder)
            .body(self.v_to_u(v.into_body()).await?)
            .map_err(Error::boxed)
    }
}
