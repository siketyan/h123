use std::fmt::{Display, Formatter};
use std::future::Future;

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
pub trait Adapter<U, V>: Send + Sync {
    async fn u_to_v(&self, u: U) -> Result<V>;
    async fn v_to_u(&self, v: V) -> Result<U>;
}

#[async_trait]
pub trait HttpAdapter<U, V>: Send + Sync {
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
    T: Adapter<U, V>,
{
    async fn u_to_v(&self, u: Request<U>) -> Result<Request<V>>
    where
        U: Send + 'async_trait,
    {
        transform_request(u, |u| self.u_to_v(u)).await
    }

    async fn v_to_u(&self, v: Response<V>) -> Result<Response<U>>
    where
        V: Send + 'async_trait,
    {
        transform_response(v, |v| self.v_to_u(v)).await
    }
}

async fn transform_request<F, Fut, U, V>(u: Request<U>, f: F) -> Result<Request<V>>
where
    F: FnOnce(U) -> Fut,
    Fut: Future<Output = Result<V>>,
{
    let mut builder = Request::builder()
        .method(u.method().clone())
        .uri(u.uri().clone())
        .version(u.version());

    for (k, v) in u.headers() {
        builder = builder.header(k, v.clone());
    }

    builder.body(f(u.into_body()).await?).map_err(Error::boxed)
}

async fn transform_response<F, Fut, V, U>(v: Response<V>, f: F) -> Result<Response<U>>
where
    F: FnOnce(V) -> Fut,
    Fut: Future<Output = Result<U>>,
{
    let mut builder = Response::builder().status(v.status()).version(v.version());

    for (k, v) in v.headers() {
        builder = builder.header(k, v.clone());
    }

    builder.body(f(v.into_body()).await?).map_err(Error::boxed)
}
