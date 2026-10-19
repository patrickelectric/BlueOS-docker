//! Keyed timers for the service loop: scheduling an armed key re-arms it, cancelling an unarmed key does nothing.

use std::{collections::HashMap, hash::Hash, time::Duration};

use tokio_util::time::{DelayQueue, delay_queue};

pub struct Timers<Key> {
    queue: DelayQueue<Key>,
    armed: HashMap<Key, delay_queue::Key>,
}

impl<Key> Default for Timers<Key> {
    fn default() -> Self {
        Self {
            queue: DelayQueue::new(),
            armed: HashMap::new(),
        }
    }
}

impl<Key: Clone + Eq + Hash> Timers<Key> {
    pub fn schedule(&mut self, key: Key, after: Duration) {
        match self.armed.get(&key) {
            Some(armed) => self.queue.reset(armed, after),
            None => {
                let armed = self.queue.insert(key.clone(), after);
                self.armed.insert(key, armed);
            }
        }
    }

    pub fn cancel(&mut self, key: &Key) {
        if let Some(armed) = self.armed.remove(key) {
            self.queue.remove(&armed);
        }
    }

    pub fn is_armed(&self, key: &Key) -> bool {
        self.armed.contains_key(key)
    }

    /// The next key whose time came. Waits forever while nothing is armed, so a `select!` branch stays pending.
    pub async fn next(&mut self) -> Key {
        match std::future::poll_fn(|context| self.queue.poll_expired(context)).await {
            Some(expired) => {
                self.armed.remove(expired.get_ref());
                expired.into_inner()
            }
            None => std::future::pending().await,
        }
    }
}
