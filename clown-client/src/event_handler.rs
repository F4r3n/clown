use color_eyre::eyre::Result;
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
    task: Option<JoinHandle<()>>,
}

impl EventHandler {
    pub fn new() -> Self {
        let tick_rate = std::time::Duration::from_millis(250);

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
            task: Some(task),
        }
    }

    pub async fn next(&mut self) -> Result<Event> {
        self.rx
            .recv()
            .await
            .ok_or(color_eyre::eyre::eyre!("Unable to get event"))
    }

    pub async fn join(&mut self) -> anyhow::Result<()> {
        if let Some(task) = self.task.take() {
            if let Err(e) = join!(task).0 {
                return Err(anyhow::Error::msg("Failed to stop"));
            }
        }
        Ok(())
    }
}
