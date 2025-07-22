use crate::event_handler::Event;
use ratatui::Frame;
use std::collections::HashMap;
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
    let mut events = EventHandler::new(); // new
    EventHandler::enable_mouse_event()?;
    let mut terminal = tui::init()?;

    let mut model = model::Model::new();
    model.save()?;
    let mut views: ViewMap = HashMap::new();
    views.insert(
        View::MainView,
        Box::new(irc_view::main_view::MainView::new()),
    );

    while model.running_state != RunningState::Done {
        let event = events.next().await?; // new
        // Render the current view
        terminal.draw(|f| view(&mut model, &mut views, f))?;

        // Handle events and map to a Message
        let mut current_msg = handle_event(&mut model, &mut views, event)?;

        // Process updates as long as they return a non-None message
        while current_msg.is_some() {
            current_msg = update(&mut model, &mut views, current_msg.unwrap()).await;
        }
    }

    tui::restore()?;
    Ok(())
}

fn view(model: &mut Model, views: &mut ViewMap, frame: &mut Frame) {
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

async fn update(model: &mut Model, views: &mut ViewMap, msg: MessageEvent) -> Option<MessageEvent> {
    if let Some(current_view) = views.get_mut(&model.current_view) {
        match msg {
            MessageEvent::Quit => {
                // You can handle cleanup and exit here
                model.running_state = RunningState::Done;
                None
            }
            _ => current_view.update(model, msg),
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {}
