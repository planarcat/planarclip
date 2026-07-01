use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub const MAX_CONCURRENT_TRANSFERS: usize = 2;

pub struct TransferSlotLimiter {
    semaphore: Arc<Semaphore>,
}

impl TransferSlotLimiter {
    pub fn new(max: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max)),
        }
    }

    pub async fn acquire(&self) -> OwnedSemaphorePermit {
        self.semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("transfer slot semaphore closed")
    }
}
