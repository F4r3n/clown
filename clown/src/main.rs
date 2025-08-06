use crate::event_handler::Event;
use crate::irc_view::message_content::MessageContent;
use ratatui::Frame;
use std::collections::HashMap;
use std::collections::VecDeque;
mod component;
mod focus_manager;
mod irc_view;
mod tui;
use model::Model;
use model::RunningState;
use model::View;
mod message_event;
use message_event::MessageEvent;

use crate::event_handler::EventHandler;
mod command;
mod config;
mod event_handler;
mod logger;
mod model;
mod widget_view;
type ViewMap = HashMap<View, Box<dyn widget_view::WidgetView>>;
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let mut events = EventHandler::new();
    EventHandler::enable_mouse_event()?;
    let mut terminal = tui::init()?;

    let mut model = model::Model::new();
    let mut views: ViewMap = HashMap::new();
    views.insert(
        View::MainView,
        Box::new(irc_view::main_view::MainView::new(&model.current_channel)),
    );
    let mut list_messages = VecDeque::new();
    list_messages.push_back(MessageEvent::AddMessageView(
        model.current_channel.clone(),
        MessageContent::new_info("Use the command /help"),
    ));

    while model.running_state != RunningState::Done {
        let event = events.next().await?;

        terminal.draw(|f| view(&mut model, &mut views, f))?;

        if let Some(message) = handle_event(&mut model, &mut views, event)? {
            list_messages.push_back(message);
        }

        while let Some(current_msg) = list_messages.pop_front() {
            update(&mut model, &mut views, current_msg, &mut list_messages).await;
        }
    }
    model.save()?;

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
) -> color_eyre::Result<Option<MessageEvent>> {
    if let Some(current_view) = views.get_mut(&model.current_view) {
        return current_view.handle_event(model, &event);
    }

    Ok(None)
}

async fn update(
    model: &mut Model,
    views: &mut ViewMap,
    msg: MessageEvent,
    out_messages: &mut VecDeque<MessageEvent>,
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
