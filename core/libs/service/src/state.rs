//! A State: published when it changes, and answered with its last value to a client that joins later.

use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use blueos_idl::{Message, cdr};
use zenoh::{Wait, query::Queryable};

use crate::{Error, Service, keys, lock};

pub struct State<M> {
    key: String,
    encoding: String,
    session: zenoh::Session,
    last: Arc<Mutex<Option<Vec<u8>>>>,
    _queryable: Queryable<()>,
    _message: PhantomData<M>,
}

impl<M: Message> State<M> {
    pub async fn declare(session: &zenoh::Session, key: String) -> Result<Self, Error> {
        let last = Arc::new(Mutex::new(None::<Vec<u8>>));
        let encoding = cdr::encoding::<M>();
        let queryable = session
            .declare_queryable(&key)
            .callback({
                let (last, key, encoding) = (Arc::clone(&last), key.clone(), encoding.clone());
                move |query| {
                    let bytes = lock(&last).clone();
                    if let Some(bytes) = bytes
                        && let Err(error) = query.reply(&key, bytes).encoding(encoding.as_str()).wait()
                    {
                        tracing::warn!(%error, key, "Failed to answer a late joiner");
                    }
                }
            })
            .await?;
        Ok(Self {
            key,
            encoding,
            session: session.clone(),
            last,
            _queryable: queryable,
            _message: PhantomData,
        })
    }

    pub async fn put_if_changed(&self, message: &M) -> Result<(), Error> {
        let bytes = cdr::encode(message)?;
        if lock(&self.last).as_deref() == Some(bytes.as_slice()) {
            return Ok(());
        }
        // Kept only once sent, so a failed put is tried again on the next change check.
        self.session
            .put(&self.key, bytes.clone())
            .encoding(self.encoding.as_str())
            .await?;
        *lock(&self.last) = Some(bytes);
        Ok(())
    }
}

impl<Arguments> Service<Arguments> {
    /// Declares a State of the route table at `blueos/v1/<service>/state/<name>`.
    pub async fn state<M: Message>(&self, name: &str) -> Result<State<M>, Error> {
        self.check_route("state", name)?;
        State::declare(&self.session, keys::state(self.name, name)).await
    }
}
