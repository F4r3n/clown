use ratatui::{
    Frame,
    crossterm::event::{Event, KeyEvent, MouseEvent},
    layout::Rect,
};
use std::ops::{Deref, DerefMut};

pub trait Draw {
    fn render(&mut self, frame: &mut Frame, area: Rect);
}

pub trait EventHandler {
    fn handle_events(&mut self, event: &Event) -> Option<Message>;
    fn handle_actions(&mut self, event: &Message) -> Option<Message>;

    fn set_focus(&mut self, focused: bool) {}
    fn has_focus(&self) -> bool;
}

pub struct Component<'a, T> {
    id: WidgetId<'a>,
    inner: T,
}

pub type WidgetId<'a> = &'a str;
use crate::Message;
impl<'a, T> Component<'a, T> {
    pub fn new(id: WidgetId<'a>, inner: T) -> Self {
        Self { id, inner }
    }

    /// Returns the unique identifier for this component
    pub fn get_id(&self) -> &WidgetId<'a> {
        &self.id
    }

    pub fn set_focus(&mut self, focused: bool)
    where
        T: EventHandler,
    {
        self.inner.set_focus(focused);
    }

    pub fn has_focus(&self) -> bool
    where
        T: EventHandler,
    {
        self.inner.has_focus()
    }

    pub fn can_focus(&self) -> bool {
        true
    }

    pub fn handle_events(&mut self, event: &Event) -> Option<Message>
    where
        T: EventHandler,
    {
        if self.inner.has_focus() {
            self.inner.handle_events(event)
        } else {
            None
        }
    }

    pub fn handle_actions(&mut self, event: &Message) -> Option<Message>
    where
        T: EventHandler,
    {
        self.inner.handle_actions(event)
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect)
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
}

pub trait ToChild {
    fn to_child_mut(&mut self) -> Child<'_>;
}

impl<T: EventHandler> ToChild for T {
    fn to_child_mut(&mut self) -> Child<'_> {
        Child::Borrowed(self)
    }
}

pub enum Child<'a> {
    Borrowed(&'a mut dyn EventHandler),
    Owned(Box<dyn 'a + EventHandler>),
}

impl<'a> Deref for Child<'a> {
    type Target = dyn 'a + EventHandler;
    fn deref(&self) -> &Self::Target {
        match self {
            Child::Borrowed(inner) => *inner,
            Child::Owned(inner) => inner.deref(),
        }
    }
}

impl<'a> DerefMut for Child<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Child::Borrowed(inner) => *inner,
            Child::Owned(inner) => inner.deref_mut(),
        }
    }
}

impl EventHandler for Child<'_> {
    fn handle_events(&mut self, event: &Event) -> Option<Message> {
        self.deref_mut().handle_events(event)
    }

    fn has_focus(&self) -> bool {
        self.deref().has_focus()
    }

    fn set_focus(&mut self, focused: bool) {
        self.deref_mut().set_focus(focused);
    }

    fn handle_actions(&mut self, event: &Message) -> Option<Message> {
        self.deref_mut().handle_actions(event)
    }
}
