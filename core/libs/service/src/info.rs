//! The `info` Query and the liveliness token every service has.

use blueos_idl::{blueos_msgs::ServiceInfo, cdr};
use zenoh::{Wait, liveliness::LivelinessToken, query::Queryable};

use crate::{Error, Service, keys, lock};

/// Keeps the service announced while it lives.
pub struct Ready {
    _info: Queryable<()>,
    _alive: LivelinessToken,
}

impl<Arguments> Service<Arguments> {
    /// Declares the `info` Query with every endpoint declared so far, and then the liveliness token, so a client
    /// that sees the service alive finds every endpoint answering. Call it once the endpoints are declared.
    pub async fn ready(&self) -> Result<Ready, Error> {
        let info = ServiceInfo {
            name: self.name.to_owned(),
            version: self.version.to_owned(),
            build: option_env!("BLUEOS_BUILD").unwrap_or("dev").to_owned(),
            capabilities: Vec::new(),
            endpoints: lock(&self.endpoints).clone(),
        };
        let bytes = cdr::encode(&info)?;
        let key = keys::query(self.name, "info");
        let queryable = self
            .session
            .declare_queryable(&key)
            .callback({
                let key = key.clone();
                move |query| {
                    let reply = query
                        .reply(&key, bytes.clone())
                        .encoding(cdr::encoding::<ServiceInfo>());
                    if let Err(error) = reply.wait() {
                        tracing::warn!(%error, key, "Failed to reply");
                    }
                }
            })
            .await?;
        let alive = self
            .session
            .liveliness()
            .declare_token(keys::liveliness(self.name))
            .await?;
        Ok(Ready {
            _info: queryable,
            _alive: alive,
        })
    }
}
