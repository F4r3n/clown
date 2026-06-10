use tokio::task::JoinHandle;
#[allow(dead_code)]
pub struct AsyncTask<T> {
    pub handle: Option<JoinHandle<anyhow::Result<T>>>,
    pub result: Option<anyhow::Result<T>>,
}

#[allow(dead_code)]
impl<T> AsyncTask<T> {
    pub fn is_ready(&self) -> bool {
        self.handle
            .as_ref()
            .map(|v| v.is_finished())
            .unwrap_or_else(|| false)
    }

    pub fn take_result(mut self) -> Option<anyhow::Result<T>> {
        self.result.take()
    }

    pub fn poll(&mut self) -> bool {
        use futures::FutureExt;
        if !self.is_ready() {
            return false;
        }

        let Some(handle) = self.handle.take() else {
            return false;
        };

        match handle.now_or_never() {
            Some(Ok(data)) => self.result = Some(data),
            _ => self.result = Some(Err(anyhow::anyhow!("Task failed"))), // Task failed, panicked, or wasn't actually finished
        };

        true
    }
}
