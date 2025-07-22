use color_eyre::eyre::Result;
use crossterm::ExecutableCommand;
use futures::{FutureExt, StreamExt, join};
use tokio::{sync::mpsc, task::JoinHandle};

#[derive(Clone, Debug)]
pub enum Event {
    Error,
    Tick,
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
    _tx: mpsc::UnboundedSender<Event>,
    rx: mpsc::UnboundedReceiver<Event>,
    _task: Option<JoinHandle<()>>,
}

impl EventHandler {
    pub fn new() -> Self {
        let tick_rate = std::time::Duration::from_millis(60);

        let (tx, rx) = mpsc::unbounded_channel();
        let _tx = tx.clone();

        let task = tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut interval = tokio::time::interval(tick_rate);
            loop {
                let delay = interval.tick();
                let crossterm_event = reader.next().fuse();
                tokio::select! {
                  maybe_event = crossterm_event => {
                    match maybe_event {
                      Some(Ok(evt)) => {
                        if tx.send(Event::Crossterm(evt)).is_err() {
                            break;
                        }
                      }
                      Some(Err(_)) => {
                        if tx.send(Event::Error).is_err() {
                            break;
                        }
                      }
                      None => {},
                    }
                  },
                  _ = delay => {
                      if tx.send(Event::Tick).is_err() {
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

    pub async fn next(&mut self) -> Result<Event> {
        self.rx
            .recv()
            .await
            .ok_or(color_eyre::eyre::eyre!("Unable to get event"))
    }

    pub async fn _join(&mut self) -> color_eyre::Result<()> {
        if let Some(task) = self._task.take() {
            if let Err(_e) = join!(task).0 {
                return Err(color_eyre::eyre::Error::msg("Failed to stop"));
            }
        }
        Ok(())
    }
}
