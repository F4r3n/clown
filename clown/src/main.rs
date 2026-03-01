mod async_task;
mod component;
mod config;
mod event_handler;
mod irc_view;
mod message_event;
mod message_irc;
mod message_queue;
mod model;
mod project_path;
mod tui;
mod widget_view;
use crate::irc_view::main_view;
use crate::irc_view::session::Session;
use clown::project_path::ProjectPath;
use event_handler::Event;
use event_handler::EventHandler;
use message_event::MessageEvent;
use message_irc::message_content::MessageKind;
use message_queue::MessageQueue;
use model::{Model, RunningState};
use ratatui::Frame;
use shadow_rs::shadow;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub enum Views<'a> {
    Main(main_view::MainView<'a>),
}
shadow!(build);

use clap::Parser;

use crate::widget_view::WidgetView;

#[derive(Parser, Debug)]
#[command(version = build::PKG_VERSION, long_version = build::VERSION)]
struct Args {
    #[arg(short, long, default_value = "clown.toml")]
    config_name: String,

    #[arg(short, long)]
    debug: bool,
}

fn prepare_logs(is_debug: bool) -> color_eyre::Result<WorkerGuard> {
    let file_appender = tracing_appender::rolling::never(
        if !is_debug {
            ProjectPath::cache_dir().unwrap_or(std::env::current_dir()?)
        } else {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        },
        ProjectPath::log_name(),
    );
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let fmt_layer = fmt::layer()
        .with_writer(non_blocking)
        .compact()
        .with_ansi(false);

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(EnvFilter::from_default_env())
        .init();
    Ok(guard)
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let args = Args::parse();
    let _guard = prepare_logs(args.debug)?;
    //info!("TEST");
    color_eyre::install()?;

    let mut model = model::Model::new(args.config_name);
    let mut session = Session::new(model.get_server_count());
    let mut current_view = Views::Main(main_view::MainView::new());

    let mut events = EventHandler::new();
    EventHandler::enable_mouse_event()?;
    let mut terminal = tui::init()?;

    let mut list_messages = message_queue::MessageQueue::new();
    list_messages.push_message(MessageEvent::AddMessageViewInfo(
        None,
        None,
        MessageKind::Info,
        "Use the command /help".to_string(),
    ));

    while model.running_state != RunningState::Done {
        if let Some(event) = events.next().await {
            match event {
                Event::Tick | Event::Crossterm(_) => {
                    handle_event(
                        &mut model,
                        &mut session,
                        &mut current_view,
                        event,
                        &mut list_messages,
                    )?;
                    while let Some(current_msg) = list_messages.next() {
                        update(
                            &mut model,
                            &mut session,
                            &mut current_view,
                            current_msg,
                            &mut list_messages,
                        )
                        .await;
                    }
                }
                Event::Error => {
                    tracing::error!("Error in the events");
                }
            }
            if need_redraw(&mut model, &mut current_view) {
                terminal.draw(|f| view(&mut model, &mut session, &mut current_view, f))?;
            }
        }
    }
    EventHandler::disable_mouse_event()?;

    tui::restore()?;
    Ok(())
}

fn view(model: &mut Model, session: &mut Session, views: &mut Views<'_>, frame: &mut Frame<'_>) {
    match views {
        Views::Main(view) => {
            view.view(model, session, frame);
        }
    }
}

fn handle_event(
    model: &mut Model,
    session: &mut Session,
    views: &mut Views<'_>,
    event: Event,
    out_messages: &mut MessageQueue,
) -> color_eyre::Result<Option<MessageEvent>> {
    match views {
        Views::Main(view) => {
            view.handle_event(model, session, &event, out_messages);
        }
    }
    Ok(None)
}

fn need_redraw(model: &mut Model, views: &mut Views<'_>) -> bool {
    match views {
        Views::Main(view) => view.need_redraw(model),
    }
}

async fn update(
    model: &mut Model,
    session: &mut Session,
    views: &mut Views<'_>,
    msg: MessageEvent,
    out_messages: &mut MessageQueue,
) {
    match views {
        Views::Main(view) => view.update(model, session, msg, out_messages),
    }
}

#[cfg(test)]
mod tests {}
