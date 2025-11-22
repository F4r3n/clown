use tokio::{
    runtime::Handle,
    task::{JoinHandle, block_in_place},
};

pub struct AsyncTask<T> {
    pub handle: Option<JoinHandle<color_eyre::Result<T>>>,
    pub on_finish: Box<dyn FnOnce(T)>,
}

impl<T> AsyncTask<T> {
    fn is_ready(&self) -> bool {
        self.handle
            .as_ref()
            .map(|v| v.is_finished())
            .unwrap_or_else(|| false)
    }

    fn poll(mut self) {
        if !self.is_ready() {
            return;
        }

        let Some(handle) = self.handle.take() else {
            return;
        };

        block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            match rt.block_on(handle) {
                Ok(Ok(value)) => (self.on_finish)(value),
                _ => {}
            }
        });
    }
}
