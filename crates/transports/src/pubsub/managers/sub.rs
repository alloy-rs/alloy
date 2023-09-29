use std::collections::BTreeMap;

use alloy_json_rpc::{EthNotification, Id, Request};
use alloy_primitives::U256;
use bimap::BiBTreeMap;
use serde_json::value::RawValue;
use tokio::sync::broadcast;

use crate::pubsub::managers::ActiveSubscription;

#[derive(Default, Debug)]
pub struct SubscriptionManager {
    /// The subscriptions.
    subs: BTreeMap<Id, ActiveSubscription>,
    /// Tracks the CURRENT server id for a subscription.
    server_ids: BiBTreeMap<U256, Id>,
    /// The alias is the FIRST server id that was used for the subscription.
    aliases: BiBTreeMap<U256, Id>,
}

impl SubscriptionManager {
    /// Get an iterator over the subscriptions.
    pub fn iter(&self) -> impl Iterator<Item = (&Id, &ActiveSubscription)> {
        self.subs.iter()
    }

    /// Get the number of subscriptions.
    pub fn len(&self) -> usize {
        self.subs.len()
    }

    /// Insert a subscription.
    pub fn insert(
        &mut self,
        request: Request<Box<RawValue>>,
        server_id: U256,
    ) -> broadcast::Receiver<Box<RawValue>> {
        let id = request.id.clone();

        let (sub, rx) = ActiveSubscription::new(request);
        self.subs.insert(id.clone(), sub);
        self.server_ids.insert(server_id, id.clone());
        self.aliases.insert(server_id, id);

        rx
    }

    /// Insert or update the server_id for a subscription.
    pub fn upsert(
        &mut self,
        request: Request<Box<RawValue>>,
        server_id: U256,
    ) -> broadcast::Receiver<Box<RawValue>> {
        if self.subs.contains_key(&request.id) {
            self.change_server_id(&request.id, server_id);
            self.get_rx(&request.id).unwrap()
        } else {
            self.insert(request, server_id)
        }
    }

    /// Calculates an alias based on the ID. This alias should be given to the
    /// waiting subscribers.
    pub fn alias_for(&self, id: &Id) -> Option<U256> {
        self.aliases.get_by_right(id).copied()
    }

    /// De-alias an alias, getting the original ID.
    pub fn dealias(&self, alias: U256) -> Option<&Id> {
        self.aliases.get_by_left(&alias)
    }

    /// Drop all server_ids.
    pub fn drop_server_ids(&mut self) {
        self.server_ids.clear();
    }

    /// Get a mutable reference to a subscription by server_id.
    pub fn sub_mut_by_server_id(&mut self, server_id: U256) -> Option<&mut ActiveSubscription> {
        let id = self.server_ids.get_by_left(&server_id)?;
        self.subs.get_mut(id)
    }

    /// Change the server_id of a subscription.
    pub fn change_server_id(&mut self, id: &Id, server_id: U256) {
        self.server_ids.insert(server_id, id.clone());
    }

    /// Remove a subscription.
    pub fn remove_sub(&mut self, id: &Id) {
        self.subs.remove(id);
        self.aliases.remove_by_right(id);
        self.server_ids.remove_by_right(id);
    }

    /// Remove a subscription by the alias.
    pub fn remove_sub_by_alias(&mut self, alias: U256) {
        if let Some((_, id)) = self.aliases.remove_by_left(&alias) {
            self.subs.remove(&id);
            self.server_ids.remove_by_right(&id);
        }
    }

    /// Remove a subscription by the server_id.
    pub fn remove_sub_by_server_id(&mut self, server_id: U256) {
        if let Some((_, id)) = self.server_ids.remove_by_left(&server_id) {
            self.subs.remove(&id);
            self.aliases.remove_by_right(&id);
        }
    }

    /// Notify the subscription channel of a new value, if the sub is known,
    /// and if any receiver exists. If the sub id is unknown, or no receiver
    /// exists, the notification is dropped.
    pub fn notify(&mut self, notification: EthNotification) {
        if let Some(sub) = self.sub_mut_by_server_id(notification.subscription) {
            sub.notify(notification.result);
        }
    }

    /// Get a receiver for a subscription.
    pub fn get_rx(&self, id: &Id) -> Option<broadcast::Receiver<Box<RawValue>>> {
        self.subs.get(id).map(|sub| sub.tx.subscribe())
    }

    /// Get a receiver for a subscription by alias.
    pub fn get_rx_by_alias(&self, alias: U256) -> Option<broadcast::Receiver<Box<RawValue>>> {
        let id = self.dealias(alias)?;
        self.get_rx(id)
    }

    /// Get a receiver for a subscription by server_id.
    pub fn get_rx_by_server_id(
        &self,
        server_id: U256,
    ) -> Option<broadcast::Receiver<Box<RawValue>>> {
        let id = self.server_ids.get_by_left(&server_id)?;
        self.get_rx(id)
    }
}
