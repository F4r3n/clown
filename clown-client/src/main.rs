use clown_core::{
    client::{Client, IRCConfig},
    conn::Connection,
};
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
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
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
            address: "chat.freenode.net".into(),
            port: 6697,
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
        // Render the current view
        terminal.draw(|f| view(&mut model, &mut views, f))?;

        // Handle events and map to a Message
        let mut current_msg = handle_event(&mut model, &mut views)?;

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

async fn update(model: &mut Model, views: &mut ViewMap, msg: Message) -> Option<Message> {
    if let Some(current_view) = views.get_mut(&model.current_view) {
        match msg {
            Message::Quit => {
                // You can handle cleanup and exit here
                model.running_state = RunningState::Done;
                None
            }
            Message::Connect => {
                if let Some(connection_config) = model.connection_config.clone() {
                    let connect = Connection::new(connection_config);
                    if let Ok(stream) = connect.connect().await {
                        if let Some(irc_config) = model.irc_config.clone() {
                            let mut client = Client::new(irc_config);
                            let reciever = client.message_receiver();
                            let command_sender = client.command_sender();

                            model.command_sender = Some(command_sender);
                            model.message_reciever = reciever;

                            client.spawn(stream);
                        }
                    } else {
                        return None;
                    }
                }

                None
            }
            _ => current_view.update(model, msg),
        }
    } else {
        if let Some(reciever) = model.message_reciever.as_mut() {
            if let Ok(recieved) = reciever.inner.try_recv() {
                if let Some(command) = recieved.get_command() {
                    match command {
                        clown_core::command::Command::PRIVMSG(_target, content) => {
                            return Some(Message::AddMessage(content));
                        }
                        _ => return None,
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {}
