use tokio::task::{JoinHandle, block_in_place};
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
        if !self.is_ready() {
            return false;
        }

        let Some(handle) = self.handle.take() else {
            return false;
        };

        let res = block_in_place(|| tokio::runtime::Handle::current().block_on(handle));

        match res {
            Ok(res) => self.result = Some(res),
            Err(e) => self.result = Some(Err(anyhow::anyhow!(e))),
        }

        true
    }
}
