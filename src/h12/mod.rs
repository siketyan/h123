mod endpoint;
mod tls;

use async_trait::async_trait;
use bytes::Bytes;
use http::header::ALT_SVC;
use http::response::Builder;
use hyper::body::to_bytes;
use hyper::Body;

use crate::convert::{Adapter, Error as ConversionError, HttpHeaderAdapter};

pub use endpoint::{Endpoint, Error};

struct BodyAdapter {
    port: u16,
}

impl BodyAdapter {
    fn new(port: u16) -> Self {
        Self { port }
    }
}

#[async_trait]
impl Adapter<Body, Bytes> for BodyAdapter {
    async fn u_to_v(&self, u: Body) -> Result<Bytes, ConversionError> {
        to_bytes(u).await.map_err(ConversionError::boxed)
    }

    async fn v_to_u(&self, v: Bytes) -> Result<Body, ConversionError> {
        Ok(Body::from(v))
    }
}

impl HttpHeaderAdapter for BodyAdapter {
    fn response_header(&self, response: Builder) -> Builder {
        response.header(
            ALT_SVC,
            format!(
                "h3=\":{}\"; ma=86400, h3-29=\":{}\"; ma=86400",
                self.port, self.port
            ),
        )
    }
}
