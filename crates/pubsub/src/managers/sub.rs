use crate::{managers::ActiveSubscription, RawSubscription};
use alloy_json_rpc::{EthNotification, SerializedRequest, SubId};
use alloy_primitives::{B256, U256};
use bimap::BiBTreeMap;
use std::collections::BTreeMap;

/// Normalizes hexadecimal string IDs to their numeric representation.
///
/// JSON-RPC subscription IDs may be encoded as either a hexadecimal string or a
/// JSON number. Servers are allowed to use either representation, so both forms
/// must address the same local subscription.
fn canonical_server_id(id: &SubId) -> SubId {
    match id {
        SubId::String(value)
            if value.starts_with("0x") || value.starts_with("0X") =>
        {
            value.parse::<U256>().map(SubId::Number).unwrap_or_else(|_| id.clone())
        }
        _ => id.clone(),
    }
}

#[derive(Debug, Default)]
pub(crate) struct SubscriptionManager {
    /// The subscriptions.
    local_to_sub: BiBTreeMap<B256, ActiveSubscription>,
    /// Tracks the current server ID for a subscription.
    local_to_server: BTreeMap<B256, SubId>,
    /// Maps canonical server IDs back to local IDs for notification routing.
    server_to_local: BTreeMap<SubId, B256>,
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
        self.set_server_id(local_id, server_id);
        self.local_to_sub.insert(local_id, active);

        sub
    }

    /// Insert or update the server ID for a subscription.
    pub(crate) fn upsert(
        &mut self,
        request: SerializedRequest,
        server_id: SubId,
        channel_size: usize,
    ) -> RawSubscription {
        let local_id = request.params_hash();

        // If we already know a subscription with the exact params,
        // we can just update the server ID and get a new listener.
        if self.local_to_sub.contains_left(&local_id) {
            self.change_server_id(local_id, server_id);
            self.get_subscription(local_id).expect("checked existence")
        } else {
            self.insert(request, server_id, channel_size)
        }
    }

    /// De-alias a server ID to its local ID.
    pub(crate) fn local_id_for(&self, server_id: &SubId) -> Option<B256> {
        self.server_to_local.get(&canonical_server_id(server_id)).copied()
    }

    /// Get the original server ID for a local ID.
    pub(crate) fn server_id_for(&self, local_id: &B256) -> Option<&SubId> {
        self.local_to_server.get(local_id)
    }

    /// Drop all server IDs.
    pub(crate) fn drop_server_ids(&mut self) {
        self.local_to_server.clear();
        self.server_to_local.clear();
    }

    /// Change the server ID for a subscription.
    fn change_server_id(&mut self, local_id: B256, server_id: SubId) {
        self.set_server_id(local_id, server_id);
    }

    fn set_server_id(&mut self, local_id: B256, server_id: SubId) {
        if let Some(previous) = self.local_to_server.insert(local_id, server_id.clone()) {
            self.server_to_local.remove(&canonical_server_id(&previous));
        }
        self.server_to_local.insert(canonical_server_id(&server_id), local_id);
    }

    /// Remove a subscription by its local ID.
    pub(crate) fn remove_sub(&mut self, local_id: B256) {
        let _ = self.local_to_sub.remove_by_left(&local_id);
        if let Some(server_id) = self.local_to_server.remove(&local_id) {
            self.server_to_local.remove(&canonical_server_id(&server_id));
        }
    }

    /// Notify the subscription channel of a new value, if the subscription is known
    /// and if any receiver exists. If the subscription ID is unknown, or no receiver
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

    #[test]
    fn matches_numeric_notifications_to_hex_server_ids() {
        let request = Request::new("eth_subscribe", Id::Number(1), ()).serialize().unwrap();
        let local_id = request.params_hash();
        let server_id = SubId::String("0x7413bf1aeb8f1c0087c36b4243f7a41a".to_owned());

        let mut manager = SubscriptionManager::default();
        manager.upsert(request, server_id.clone(), 1);

        let notification_id = SubId::Number("0x7413bf1aeb8f1c0087c36b4243f7a41a".parse().unwrap());

        assert_eq!(manager.local_id_for(&notification_id), Some(local_id));
        assert_eq!(manager.server_id_for(&local_id), Some(&server_id));
    }

    #[test]
    fn replaces_old_server_id_lookup() {
        let request = Request::new("eth_subscribe", Id::Number(1), ()).serialize().unwrap();
        let local_id = request.params_hash();

        let mut manager = SubscriptionManager::default();
        manager.upsert(request.clone(), SubId::String("0x01".to_owned()), 1);
        manager.upsert(request, SubId::String("0x02".to_owned()), 1);

        assert_eq!(
            manager.local_id_for(&SubId::Number("0x01".parse().unwrap())),
            None
        );
        assert_eq!(
            manager.local_id_for(&SubId::Number("0x02".parse().unwrap())),
            Some(local_id)
        );
    }
}
