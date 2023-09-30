use alloy_json_rpc::EthNotification;
use alloy_primitives::{keccak256, B256, U256};
use bimap::BiBTreeMap;
use serde_json::value::RawValue;
use tokio::sync::broadcast;

use crate::pubsub::managers::ActiveSubscription;

#[derive(Default, Debug)]
pub struct SubscriptionManager {
    /// The subscriptions.
    local_to_sub: BiBTreeMap<B256, ActiveSubscription>,
    /// Tracks the CURRENT server id for a subscription.
    server_to_local: BiBTreeMap<U256, B256>,
}

impl SubscriptionManager {
    /// Get an iterator over the subscriptions.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&B256, &ActiveSubscription)> {
        self.local_to_sub.iter()
    }

    /// Get the number of subscriptions.
    pub fn len(&self) -> usize {
        self.local_to_sub.len()
    }

    /// Insert a subscription.
    pub fn insert(
        &mut self,
        request: Box<RawValue>,
        server_id: U256,
    ) -> broadcast::Receiver<Box<RawValue>> {
        let (sub, rx) = ActiveSubscription::new(request);
        self.server_to_local.insert(server_id, sub.local_id);
        self.local_to_sub.insert(sub.local_id, sub);

        rx
    }

    /// Insert or update the server_id for a subscription.
    pub fn upsert(
        &mut self,
        request: Box<RawValue>,
        server_id: U256,
    ) -> broadcast::Receiver<Box<RawValue>> {
        let local_id = keccak256(request.get());

        // If we already know a subscription with the exact params,
        // we can just update the server_id and get a new listener.
        if self.server_to_local.contains_right(&local_id) {
            self.change_server_id(server_id, local_id);
            self.get_rx(local_id).expect("checked existence")
        } else {
            self.insert(request, server_id)
        }
    }

    /// De-alias an alias, getting the original ID.
    pub fn local_id_for(&self, server_id: U256) -> Option<B256> {
        self.server_to_local.get_by_left(&server_id).copied()
    }

    /// Drop all server_ids.
    pub fn drop_server_ids(&mut self) {
        self.server_to_local.clear();
    }

    /// Change the server_id of a subscription.
    pub fn change_server_id(&mut self, server_id: U256, local_id: B256) {
        self.server_to_local.insert(server_id, local_id);
    }

    /// Remove a subscription by its local_id.
    pub fn remove_sub(&mut self, local_id: B256) {
        let _ = self.local_to_sub.remove_by_left(&local_id);
        let _ = self.server_to_local.remove_by_right(&local_id);
    }

    /// Notify the subscription channel of a new value, if the sub is known,
    /// and if any receiver exists. If the sub id is unknown, or no receiver
    /// exists, the notification is dropped.
    pub fn notify(&mut self, notification: EthNotification) {
        if let Some(local_id) = self.local_id_for(notification.subscription) {
            if let Some((_, mut sub)) = self.local_to_sub.remove_by_left(&local_id) {
                sub.notify(notification.result);
                self.local_to_sub.insert(local_id, sub);
            }
        }
    }

    /// Get a receiver for a subscription.
    pub fn get_rx(&self, local_id: B256) -> Option<broadcast::Receiver<Box<RawValue>>> {
        let sub = self.local_to_sub.get_by_left(&local_id)?;
        Some(sub.tx.subscribe())
    }
}
