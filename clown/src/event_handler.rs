use crossterm::ExecutableCommand;
use futures::{FutureExt, StreamExt, join};
use tokio::{sync::mpsc, task::JoinHandle};

#[derive(Clone, Debug)]
pub enum Event {
    Error,
    Tick,
    Redraw,
    Crossterm(crossterm::event::Event),
}

impl Event {
    pub fn get_key(&self) -> Option<crossterm::event::KeyEvent> {
        match self {
            Self::Crossterm(event) => event.as_key_event(),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct EventHandler {
    _tx: mpsc::Sender<Event>,
    rx: mpsc::Receiver<Event>,
    _task: Option<JoinHandle<()>>,
}

impl EventHandler {
    pub fn new() -> Self {
        let mut tick_interval = tokio::time::interval(std::time::Duration::from_millis(100));
        let mut redraw_interval = tokio::time::interval(std::time::Duration::from_millis(16));
        let (tx, rx) = mpsc::channel(100);
        let _tx = tx.clone();

        let task = tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();

            loop {
                let crossterm_event = reader.next().fuse();
                tokio::select! {
                  //  biased;
                  maybe_event = crossterm_event => {
                    match maybe_event {
                      Some(Ok(evt)) => {
                        if tx.send(Event::Crossterm(evt)).await.is_err() {
                            break;
                        }
                      }
                      Some(Err(_)) => {
                        if tx.send(Event::Error).await.is_err() {
                            break;
                        }
                      }
                      None => {},
                    }
                  },
                  _tick = tick_interval.tick() => {
                      if tx.send(Event::Tick).await.is_err() {
                        break;
                      }
                  },

                  _redraw = redraw_interval.tick() => {
                      if tx.send(Event::Redraw).await.is_err() {
                        break;
                      }
                  },
                }
            }
        });

        Self {
            _tx,
            rx,
            _task: Some(task),
        }
    }

    pub fn enable_mouse_event() -> color_eyre::Result<()> {
        std::io::stdout().execute(crossterm::event::EnableMouseCapture)?;
        Ok(())
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    pub async fn _join(&mut self) -> color_eyre::Result<()> {
        if let Some(task) = self._task.take()
            && let Err(_e) = join!(task).0
        {
            return Err(color_eyre::eyre::Error::msg("Failed to stop"));
        }
        Ok(())
    }

    pub fn disable_mouse_event() -> color_eyre::Result<()> {
        std::io::stdout().execute(crossterm::event::DisableMouseCapture)?;
        Ok(())
    }
}
