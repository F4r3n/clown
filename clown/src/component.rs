use crate::event_handler::Event;
use crate::message_event::MessageEvent;
use ratatui::{Frame, layout::Rect};
use std::ops::{Deref, DerefMut};

pub trait Draw {
    fn render(&mut self, frame: &mut Frame<'_>, area: Rect);
}

pub trait EventHandler {
    fn handle_events(&mut self, event: &Event) -> Option<MessageEvent>;
    fn handle_actions(&mut self, event: &MessageEvent) -> Option<MessageEvent>;
    fn need_redraw(&self) -> bool;

    fn get_area(&self) -> Rect;
}

pub struct Component<'a, T> {
    id: WidgetId<'a>,
    inner: T,
}

pub type WidgetId<'a> = &'a str;

impl<'a, T> Component<'a, T> {
    pub fn new(id: WidgetId<'a>, inner: T) -> Self {
        Self { id, inner }
    }

    /// Returns the unique identifier for this component
    pub fn get_id(&self) -> &WidgetId<'a> {
        &self.id
    }

    pub fn handle_events(&mut self, event: &Event) -> Option<MessageEvent>
    where
        T: EventHandler,
    {
        self.inner.handle_events(event)
    }

    pub fn need_redraw(&self) -> bool
    where
        T: EventHandler,
    {
        self.inner.need_redraw()
    }

    pub fn handle_actions(&mut self, event: &MessageEvent) -> Option<MessageEvent>
    where
        T: EventHandler,
    {
        self.inner.handle_actions(event)
    }

    pub fn render(&mut self, frame: &mut Frame<'_>, area: Rect)
    where
        T: Draw,
    {
        self.inner.render(frame, area);
    }

    pub fn to_child_mut<'b>(&'b mut self) -> Component<'b, Child<'b>>
    where
        T: ToChild,
    {
        Component {
            id: self.id,
            inner: self.inner.to_child_mut(),
        }
    }
    pub fn get_area(&self) -> Rect
    where
        T: EventHandler,
    {
        self.inner.get_area()
    }
}

pub trait ToChild {
    fn to_child_mut(&mut self) -> Child<'_>;
}

impl<T: EventHandler> ToChild for T {
    fn to_child_mut(&mut self) -> Child<'_> {
        Child(self)
    }
}

pub struct Child<'a>(&'a mut dyn EventHandler);

impl<'a> Deref for Child<'a> {
    type Target = dyn 'a + EventHandler;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl DerefMut for Child<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl EventHandler for Child<'_> {
    fn handle_events(&mut self, event: &Event) -> Option<MessageEvent> {
        self.deref_mut().handle_events(event)
    }

    fn get_area(&self) -> Rect {
        self.deref().get_area()
    }

    fn handle_actions(&mut self, event: &MessageEvent) -> Option<MessageEvent> {
        self.deref_mut().handle_actions(event)
    }

    fn need_redraw(&self) -> bool {
        self.deref().need_redraw()
    }
}
