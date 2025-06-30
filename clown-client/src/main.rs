use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode},
};
use std::{collections::HashMap, time::Duration};

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
mod command;
mod main_view;
mod model;
mod widget_view;
type ViewMap = HashMap<View, Box<dyn widget_view::WidgetView>>;
fn main() -> color_eyre::Result<()> {
    let mut terminal = tui::init()?;

    let option = clown_core::conn::ConnectionConfig {
        address: "chat.freenode.net".into(),
        nickname: "farine".into(),
        password: None,
        port: 6697,
        real_name: "farine".into(),
        username: "farine".into(),
    };

    let mut model = model::Model::new(option);
    let mut views: ViewMap = HashMap::new();
    views.insert(View::MainView, Box::new(main_view::MainView::new()));

    while model.running_state != RunningState::Done {
        // Render the current view
        terminal.draw(|f| view(&mut model, &mut views, f))?;

        // Handle events and map to a Message
        let mut current_msg = handle_event(&mut model, &mut views)?;

        // Process updates as long as they return a non-None message
        while current_msg.is_some() {
            current_msg = update(&mut model, &mut views, current_msg.unwrap());
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

/// Convert Event to Message
///
/// We don't need to pass in a `model` to this function in this example
/// but you might need it as your project evolves
fn handle_event(model: &mut Model, views: &mut ViewMap) -> color_eyre::Result<Option<Message>> {
    if event::poll(Duration::from_millis(250))? {
        let event = event::read()?;
        if let Some(current_view) = views.get_mut(&model.current_view) {
            return current_view.handle_event(model, &event);
        }
        if let Event::Key(key) = event {
            if key.kind == event::KeyEventKind::Press {
                return Ok(handle_key(key));
            }
        }
    }
    Ok(None)
}

fn handle_key(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc => Some(Message::Quit),
        // Tab navigation is now handled in the view, so we don't need to handle it here
        _ => None,
    }
}

fn update(model: &mut Model, views: &mut ViewMap, msg: Message) -> Option<Message> {
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
