mod async_task;
mod command;
mod component;
mod config;
mod event_handler;
mod irc_view;
mod message_event;
mod message_queue;
mod model;
mod project_path;
mod tui;
mod widget_view;

use ahash::AHashMap;
use clown::project_path::ProjectPath;
use event_handler::Event;
use event_handler::EventHandler;
use irc_view::message_content::MessageContent;
use message_event::MessageEvent;
use message_queue::MessageQueue;
use model::{Model, RunningState, View};
use ratatui::Frame;
use shadow_rs::shadow;
use tracing::{debug, error};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
shadow!(build);

use clap::Parser;

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

type ViewMap = AHashMap<View, Box<dyn widget_view::WidgetView>>;
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let args = Args::parse();
    let _guard = prepare_logs(args.debug)?;
    //info!("TEST");
    color_eyre::install()?;

    let mut model = model::Model::new(args.config_name);
    let mut views: ViewMap = AHashMap::new();
    views.insert(
        View::MainView,
        Box::new(irc_view::main_view::MainView::new(&model.current_channel)),
    );

    let mut events = EventHandler::new();
    EventHandler::enable_mouse_event()?;
    let mut terminal = tui::init()?;

    let mut list_messages = message_queue::MessageQueue::new();
    list_messages.push_message(MessageEvent::AddMessageView(
        None,
        MessageContent::new_info("Use the command /help".to_string()),
    ));

    while model.running_state != RunningState::Done {
        if let Some(event) = events.next().await {
            if need_redraw(&mut model, &mut views) {
                //debug!("Need redraw");
                terminal.draw(|f| view(&mut model, &mut views, f))?;
            }
            //debug!("{:?}", &event);
            handle_event(&mut model, &mut views, event, &mut list_messages)?;
            while let Some(current_msg) = list_messages.next() {
                update(&mut model, &mut views, current_msg, &mut list_messages).await;
            }
        }
    }
    EventHandler::disable_mouse_event()?;

    tui::restore()?;
    Ok(())
}

fn view(model: &mut Model, views: &mut ViewMap, frame: &mut Frame<'_>) {
    if let Some(current_view) = views.get_mut(&model.current_view) {
        current_view.view(model, frame);
    }
}

fn handle_event(
    model: &mut Model,
    views: &mut ViewMap,
    event: Event,
    out_messages: &mut MessageQueue,
) -> color_eyre::Result<Option<MessageEvent>> {
    if let Some(current_view) = views.get_mut(&model.current_view) {
        current_view.handle_event(model, &event, out_messages);
    }

    Ok(None)
}

fn need_redraw(model: &mut Model, views: &mut ViewMap) -> bool {
    if let Some(current_view) = views.get_mut(&model.current_view) {
        current_view.need_redraw(model)
    } else {
        false
    }
}

async fn update(
    model: &mut Model,
    views: &mut ViewMap,
    msg: MessageEvent,
    out_messages: &mut MessageQueue,
) {
    if let Some(current_view) = views.get_mut(&model.current_view) {
        match msg {
            MessageEvent::Quit(_) => {
                model.running_state = RunningState::Done;
            }
            _ => current_view.update(model, msg, out_messages),
        }
    }
}

#[cfg(test)]
mod tests {}
