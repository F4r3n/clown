mod message_event;
mod message_queue;
mod model;
use ahash::AHashMap;
use irc_view::message_content::MessageContent;
use message_event::MessageEvent;
use message_queue::MessageQueue;
use model::Model;
use model::RunningState;
use model::View;
mod component;
mod event_handler;
mod widget_view;
use crate::event_handler::EventHandler;
mod config;
mod irc_view;
mod tui;
use event_handler::Event;
use ratatui::Frame;
mod async_task;
mod command;
type ViewMap = AHashMap<View, Box<dyn widget_view::WidgetView>>;
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let file_appender = tracing_appender::rolling::never(".", "app.log");

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt().with_writer(non_blocking).init();

    color_eyre::install()?;
    let mut events = EventHandler::new();
    EventHandler::enable_mouse_event()?;
    let mut terminal = tui::init()?;

    let mut model = model::Model::new();
    let mut views: ViewMap = AHashMap::new();
    views.insert(
        View::MainView,
        Box::new(irc_view::main_view::MainView::new(&model.current_channel)),
    );
    let mut list_messages = message_queue::MessageQueue::new();
    list_messages.push_message(MessageEvent::AddMessageView(
        model.current_channel.clone(),
        MessageContent::new_info("Use the command /help".to_string()),
    ));

    while model.running_state != RunningState::Done {
        let event = events.next().await?;

        terminal.draw(|f| view(&mut model, &mut views, f))?;

        handle_event(&mut model, &mut views, event, &mut list_messages)?;

        while let Some(current_msg) = list_messages.next() {
            update(&mut model, &mut views, current_msg, &mut list_messages).await;
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
