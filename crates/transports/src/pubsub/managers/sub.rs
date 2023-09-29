use std::collections::BTreeMap;

use alloy_json_rpc::{EthNotification, Id, Request};
use alloy_primitives::U256;
use bimap::BiBTreeMap;
use serde_json::value::RawValue;
use tokio::sync::broadcast;

use crate::pubsub::managers::ActiveSubscription;

#[derive(Default, Debug)]
pub struct SubscriptionManager {
    subs: BTreeMap<Id, ActiveSubscription>,
    server_ids: BiBTreeMap<U256, Id>,
}

impl SubscriptionManager {
    /// Insert a subscription.
    pub fn insert(
        &mut self,
        request: Request<Box<RawValue>>,
        server_id: U256,
    ) -> broadcast::Receiver<Box<RawValue>> {
        let id = request.id.clone();

        let (sub, rx) = ActiveSubscription::new(request);
        self.subs.insert(id.clone(), sub);
        self.server_ids.insert(server_id, id);

        rx
    }

    /// Calculates an alias based on the ID. This alias is given to the waiting
    /// subscribers.
    pub fn alias(&self, server_id: U256) -> Option<U256> {
        let id = self.server_ids.get_by_left(&server_id)?;
        match id {
            Id::Number(n) => Some(U256::from(*n)),
            Id::String(s) => {
                let mut buf = [0u8; 32];
                let cap = if s.len() > 32 { 32 } else { s.len() };
                buf[..cap].copy_from_slice(s.as_bytes());
                Some(U256::from_be_bytes::<32>(buf))
            }
            Id::None => None,
        }
    }

    /// Get a ref to the server_id bimap.
    pub fn server_ids(&self) -> &BiBTreeMap<U256, Id> {
        &self.server_ids
    }

    /// Drop all server_ids.
    pub fn drop_server_ids(&mut self) {
        self.server_ids.clear();
    }

    /// Get a reference to a subscription.
    pub fn sub(&self, id: &Id) -> Option<&ActiveSubscription> {
        self.subs.get(id)
    }

    /// Get a mutable reference to a subscription.
    pub fn sub_mut(&mut self, id: &Id) -> Option<&mut ActiveSubscription> {
        self.subs.get_mut(id)
    }

    /// Get a mutable reference to a subscription by server_id.
    pub fn sub_mut_by_server_id(&mut self, server_id: U256) -> Option<&mut ActiveSubscription> {
        if let Some(id) = self.server_ids.get_by_left(&server_id) {
            self.subs.get_mut(id)
        } else {
            None
        }
    }

    /// Change the server_id of a subscription.
    pub fn change_server_id(&mut self, id: &Id, server_id: U256) {
        self.server_ids.insert(server_id, id.clone());
    }

    /// Get sub params.
    pub fn params(&self, id: &Id) -> Option<&serde_json::value::RawValue> {
        self.subs.get(id).map(|sub| sub.params())
    }

    /// Get sub params by server_id.
    pub fn params_by_server_id(&self, server_id: U256) -> Option<&serde_json::value::RawValue> {
        if let Some(id) = self.server_ids.get_by_left(&server_id) {
            self.params(id)
        } else {
            None
        }
    }

    /// Remove a subscription.
    pub fn remove_sub(&mut self, id: &Id) {
        self.subs.remove(id);
    }

    /// Remove a subscription and the alias.
    pub fn remove_sub_and_alias(&mut self, alias: U256) {
        if let Some((_, id)) = self.server_ids.remove_by_left(&alias) {
            self.subs.remove(&id);
        }
    }

    /// Send a notification. Remove the sub and alias if sending fails.
    pub fn forward_notification(
        &mut self,
        notification: EthNotification<Box<serde_json::value::RawValue>>,
    ) -> Result<(), broadcast::error::SendError<Box<RawValue>>> {
        if let Some(sub) = self.sub_mut_by_server_id(notification.subscription) {
            let res = sub.notify(notification.result);
            if res.is_err() {
                self.remove_sub_and_alias(notification.subscription);
            }
            return res;
        }
        Ok(())
    }
}
