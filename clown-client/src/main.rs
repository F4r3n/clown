use crate::event_handler::Event;
use clown_core::client::IRCConfig;
use ratatui::Frame;
use std::collections::HashMap;

mod component;
mod focus_manager;
mod input_widget;
mod text_widget;
mod tui;
use model::Model;
use model::RunningState;
use model::View;
mod message;
use message::Message;

use crate::event_handler::EventHandler;
mod command;
mod event_handler;
mod main_view;
mod model;
mod widget_view;
type ViewMap = HashMap<View, Box<dyn widget_view::WidgetView>>;
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let mut events = EventHandler::new(); // new
    let mut terminal = tui::init()?;

    /*
        let connect = Connection::new(option);
    connect.connect();

    let client = Client::new(IRCConfig {
        nickname: "farine".into(),
        password: None,
        real_name: "farine".into(),
        username: "farine".into(),
    });
     */

    let mut model = model::Model::new(
        Some(clown_core::conn::ConnectionConfig {
            address: "localhost".into(),
            port: 6667,
        }),
        Some(IRCConfig {
            nickname: "farine".into(),
            password: None,
            real_name: "farine".into(),
            username: "farine".into(),
            channel: "#rust-spam".into(),
        }),
    );
    let mut views: ViewMap = HashMap::new();
    views.insert(View::MainView, Box::new(main_view::MainView::new()));

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
    /*events
        .join()
        .await
        .map_err(|e| color_eyre::eyre::eyre!(e))?;
    */
    tui::restore()?;
    Ok(())
}

fn view(model: &mut Model, views: &mut ViewMap, frame: &mut Frame) {
    if let Some(current_view) = views.get_mut(&model.current_view) {
        current_view.view(model, frame);
    }
}

/// Convert Event to Message
///
/// We don't need to pass in a `model` to this function in this example
/// but you might need it as your project evolves
fn handle_event(
    model: &mut Model,
    views: &mut ViewMap,
    event: Event,
) -> color_eyre::Result<Option<Message>> {
    if let Some(current_view) = views.get_mut(&model.current_view) {
        return current_view.handle_event(model, &event);
    }

    Ok(None)
}

async fn update(model: &mut Model, views: &mut ViewMap, msg: Message) -> Option<Message> {
    if let Some(current_view) = views.get_mut(&model.current_view) {
        match msg {
            Message::Quit => {
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
