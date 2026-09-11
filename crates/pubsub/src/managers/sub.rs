use crate::{managers::ActiveSubscription, RawSubscription};
use alloy_json_rpc::{EthNotification, SerializedRequest, SubId};
use alloy_primitives::{map::HashSet, B256};
use bimap::BiBTreeMap;

#[derive(Debug, Default)]
pub(crate) struct SubscriptionManager {
    /// The subscriptions.
    local_to_sub: BiBTreeMap<B256, ActiveSubscription>,
    /// Tracks the CURRENT server id for a subscription.
    local_to_server: BiBTreeMap<B256, SubId>,
    /// Tracks subscriptions that were explicitly cancelled while a subscribe
    /// response might still be in-flight.
    cancelled: HashSet<B256>,
}

impl SubscriptionManager {
    /// Get an iterator over the subscriptions.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&B256, &ActiveSubscription)> {
        self.local_to_sub.iter()
    }

    /// Get the number of subscriptions.
    pub(crate) fn len(&self) -> usize {
        self.local_to_sub.len()
    }

    /// Insert a subscription.
    fn insert(
        &mut self,
        request: SerializedRequest,
        server_id: SubId,
        channel_size: usize,
    ) -> RawSubscription {
        let active = ActiveSubscription::new(request, channel_size);
        let sub = active.subscribe();

        let local_id = active.local_id;
        self.local_to_server.insert(local_id, server_id);
        self.local_to_sub.insert(local_id, active);

        sub
    }

    /// Insert or update the server_id for a subscription.
    pub(crate) fn upsert(
        &mut self,
        request: SerializedRequest,
        server_id: SubId,
        channel_size: usize,
    ) -> Option<RawSubscription> {
        let local_id = request.params_hash();

        if self.cancelled.contains(&local_id) {
            return None;
        }

        // If we already know a subscription with the exact params,
        // we can just update the server_id and get a new listener.
        if self.local_to_sub.contains_left(&local_id) {
            self.change_server_id(local_id, server_id);
            self.get_subscription(local_id)
        } else {
            Some(self.insert(request, server_id, channel_size))
        }
    }

    /// De-alias an alias, getting the original ID.
    pub(crate) fn local_id_for(&self, server_id: &SubId) -> Option<B256> {
        self.local_to_server.get_by_right(server_id).copied()
    }

    /// De-alias an alias, getting the original ID.
    pub(crate) fn server_id_for(&self, local_id: &B256) -> Option<&SubId> {
        self.local_to_server.get_by_left(local_id)
    }

    /// Drop all server_ids.
    pub(crate) fn drop_server_ids(&mut self) {
        self.local_to_server.clear();
    }

    /// Change the server_id of a subscription.
    fn change_server_id(&mut self, local_id: B256, server_id: SubId) {
        self.local_to_server.insert(local_id, server_id);
    }

    /// Remove a subscription by its local_id.
    pub(crate) fn remove_sub(&mut self, local_id: B256) {
        self.cancelled.insert(local_id);
        let _ = self.local_to_sub.remove_by_left(&local_id);
        let _ = self.local_to_server.remove_by_left(&local_id);
    }

    /// Clear an explicit cancellation marker for this local_id.
    ///
    /// This should be called before issuing a new subscribe request with the
    /// same params hash.
    pub(crate) fn clear_cancelled(&mut self, local_id: B256) {
        self.cancelled.remove(&local_id);
    }

    /// Notify the subscription channel of a new value, if the sub is known,
    /// and if any receiver exists. If the sub id is unknown, or no receiver
    /// exists, the notification is dropped.
    pub(crate) fn notify(&mut self, notification: EthNotification) {
        if let Some(local_id) = self.local_id_for(&notification.subscription) {
            if let Some(sub) = self.local_to_sub.get_by_left(&local_id) {
                sub.notify(notification.result);
            }
        }
    }

    /// Get a receiver for a subscription.
    pub(crate) fn get_subscription(&self, local_id: B256) -> Option<RawSubscription> {
        self.local_to_sub.get_by_left(&local_id).map(ActiveSubscription::subscribe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_json_rpc::{Id, Request};

    fn subscribe_request(id: u64) -> SerializedRequest {
        Request::new("eth_subscribe", Id::Number(id), ["newHeads"])
            .serialize()
            .expect("serializable subscribe request")
    }

    #[test]
    fn cancelled_subscription_rejects_late_upsert_until_cleared() {
        let mut manager = SubscriptionManager::default();
        let req = subscribe_request(1);
        let local_id = req.params_hash();

        let inserted = manager.upsert(req.clone(), SubId::String("0x1".to_string()), 16);
        assert!(inserted.is_some());
        assert!(manager.get_subscription(local_id).is_some());

        manager.remove_sub(local_id);
        assert!(manager.get_subscription(local_id).is_none());

        // Simulate a late subscribe response arriving after explicit cancel.
        let late = manager.upsert(req.clone(), SubId::String("0x2".to_string()), 16);
        assert!(late.is_none());
        assert!(manager.get_subscription(local_id).is_none());

        // A fresh subscribe attempt with the same params should work once the
        // tombstone is cleared.
        manager.clear_cancelled(local_id);
        let fresh = manager.upsert(req, SubId::String("0x3".to_string()), 16);
        assert!(fresh.is_some());
        assert!(manager.get_subscription(local_id).is_some());
    }
}
