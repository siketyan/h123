mod connection;
mod endpoint;

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::Mutex;

use crate::convert::{Adapter, Error as ConversionError};

pub use endpoint::{Endpoint, Error};

#[derive(Debug, thiserror::Error)]
enum AdapterError {
    #[error("Mutex error.")]
    Mutex,
}

struct BodyAdapter {
    buffer: Arc<Mutex<Bytes>>,
}

impl BodyAdapter {
    fn new(bytes: Bytes) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(bytes)),
        }
    }

    fn as_inner(&self) -> Result<Bytes, ConversionError> {
        Ok(Arc::try_unwrap(Arc::clone(&self.buffer))
            .map_err(|_| ConversionError::boxed(AdapterError::Mutex))?
            .into_inner())
    }

    fn into_inner(self) -> Result<Bytes, ConversionError> {
        self.as_inner()
    }
}

#[async_trait]
impl Adapter<(), Bytes> for BodyAdapter {
    async fn u_to_v(&self, _: ()) -> Result<Bytes, ConversionError> {
        Ok(self.buffer.lock().await.clone())
    }

    async fn v_to_u(&self, v: Bytes) -> Result<(), ConversionError> {
        let buffer = Arc::clone(&self.buffer);
        *buffer.lock().await = v;
        Ok(())
    }
}
