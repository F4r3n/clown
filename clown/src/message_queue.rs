use std::{
    cmp::Reverse,
    collections::{BinaryHeap, VecDeque},
};

use crate::message_event::MessageEvent;
use std::time::Duration;

struct TimedMessage {
    event: MessageEvent,
    time: std::time::Instant,
}

impl PartialEq for TimedMessage {
    fn eq(&self, other: &Self) -> bool {
        self.time.eq(&other.time)
    }
}

impl PartialOrd for TimedMessage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for TimedMessage {}

impl Ord for TimedMessage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

pub struct MessageQueue {
    qnow: VecDeque<MessageEvent>,
    qtimed: BinaryHeap<Reverse<TimedMessage>>,
}

impl std::iter::Iterator for MessageQueue {
    type Item = MessageEvent;
    fn next(&mut self) -> Option<Self::Item> {
        if !self.qtimed.is_empty()
            && let Some(item) = self.qtimed.peek()
            && std::time::Instant::now() > item.0.time
        {
            return self.qtimed.pop().map(|reverved| reverved.0.event);
        }

        self.qnow.pop_front()
    }
}

impl MessageQueue {
    pub fn new() -> Self {
        Self {
            qnow: VecDeque::new(),
            qtimed: BinaryHeap::new(),
        }
    }
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.qnow.len() + self.qtimed.len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.qnow.is_empty() && self.qtimed.is_empty()
    }

    pub fn push_message_with_time(&mut self, event: MessageEvent, duration: Duration) {
        self.qtimed.push(Reverse(TimedMessage {
            event,
            time: std::time::Instant::now()
                .checked_add(duration)
                .unwrap_or_else(std::time::Instant::now),
        }));
    }

    pub fn push_message(&mut self, event: MessageEvent) {
        self.qnow.push_back(event);
    }
}

#[cfg(test)]
mod test {
    use crate::message_event::MessageEvent;
    use crate::message_queue::MessageQueue;
    #[test]
    pub fn push_test() {
        let mut message_queue = MessageQueue::new();
        assert!(message_queue.is_empty());

        message_queue.push_message(MessageEvent::Connect);
        assert!(!message_queue.is_empty());

        message_queue.push_message(MessageEvent::Connect);
        message_queue.push_message(MessageEvent::Connect);
        message_queue.push_message_with_time(
            crate::message_event::MessageEvent::Connect,
            std::time::Duration::from_secs(5),
        );

        assert_eq!(message_queue.len(), 4);
    }

    #[test]
    pub fn test_next() {
        let mut message_queue = MessageQueue::new();
        message_queue.push_message(MessageEvent::Connect);
        message_queue.push_message(MessageEvent::Connect);
        message_queue.push_message(MessageEvent::Connect);
        message_queue
            .push_message_with_time(MessageEvent::Connect, std::time::Duration::from_secs(5));

        assert_eq!(message_queue.len(), 4);
        assert_eq!(message_queue.next(), Some(MessageEvent::Connect));
        assert_eq!(message_queue.next(), Some(MessageEvent::Connect));
        assert_eq!(message_queue.next(), Some(MessageEvent::Connect));
        assert_eq!(message_queue.next(), None);
    }
}
