mod endpoint;
mod tls;

use async_trait::async_trait;
use bytes::Bytes;
use hyper::body::to_bytes;
use hyper::Body;

use crate::convert::{Adapter, Error as ConversionError};

pub use endpoint::{Endpoint, Error};

struct BodyAdapter;

#[async_trait]
impl Adapter<Body, Bytes> for BodyAdapter {
    async fn u_to_v(&self, u: Body) -> Result<Bytes, ConversionError> {
        to_bytes(u).await.map_err(ConversionError::boxed)
    }

    async fn v_to_u(&self, v: Bytes) -> Result<Body, ConversionError> {
        Ok(Body::from(v))
    }
}
