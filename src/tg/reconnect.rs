use crate::tg::Backend;
use log::{debug, error};

pub(crate) struct HomoReconnectPolicy;

impl Backend {
    #[inline]
    pub async fn reconnect(&self) -> bool {
        match self.client.is_authorized().await {
            Ok(is_authorized) => {
                debug!("Reconnected with is_authorized: {is_authorized}");
                true
            }
            Err(e) => {
                error!("Reconnect failed: {e}");
                false
            }
        }
    }
}
