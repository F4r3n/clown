use tokio::task::{JoinHandle, block_in_place};
pub struct AsyncTask<T> {
    pub handle: Option<JoinHandle<color_eyre::Result<T>>>,
    pub result: Option<T>,
}

impl<T> AsyncTask<T> {
    pub fn is_ready(&self) -> bool {
        self.handle
            .as_ref()
            .map(|v| v.is_finished())
            .unwrap_or_else(|| false)
    }

    pub fn take_result(mut self) -> Option<T> {
        self.result.take()
    }

    pub fn poll(&mut self) -> bool {
        if !self.is_ready() {
            return false;
        }

        let Some(handle) = self.handle.take() else {
            return false;
        };

        self.result = block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            match rt.block_on(handle) {
                Ok(Ok(value)) => Some(value),
                _ => None,
            }
        });
        true
    }
}
