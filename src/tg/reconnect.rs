use crate::tg::Backend;
use anyhow::Result;
use grammers_mtsender::ReconnectionPolicy;
use log::{debug, error};
use std::ops::ControlFlow;
use std::time::Duration;

pub(crate) struct HomoReconnectPolicy;

impl ReconnectionPolicy for HomoReconnectPolicy {
    fn should_retry(&self, attempts: usize) -> ControlFlow<(), Duration> {
        debug!("Reconnecting attempt {}", attempts);
        // retry after 1 second for developing phase
        ControlFlow::Continue(Duration::from_secs(1))
    }
}

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
