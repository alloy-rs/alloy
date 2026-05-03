use crate::{managers::ActiveSubscription, RawSubscription};
use alloy_json_rpc::{EthNotification, SerializedRequest, SubId};
use alloy_primitives::B256;
use bimap::BiBTreeMap;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub(crate) struct SubscriptionManager {
    /// The subscriptions.
    local_to_sub: BiBTreeMap<B256, ActiveSubscription>,
    /// Tracks the CURRENT server id for a subscription.
    local_to_server: BiBTreeMap<B256, SubId>,
    /// Tracks all server id aliases that may identify a subscription in notifications.
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
        self.insert_server_id(local_id, server_id);
        self.local_to_sub.insert(local_id, active);

        sub
    }

    /// Insert or update the server_id for a subscription.
    pub(crate) fn upsert(
        &mut self,
        request: SerializedRequest,
        server_id: SubId,
        channel_size: usize,
    ) -> RawSubscription {
        let local_id = request.params_hash();

        // If we already know a subscription with the exact params,
        // we can just update the server_id and get a new listener.
        if self.local_to_sub.contains_left(&local_id) {
            self.change_server_id(local_id, server_id);
            self.get_subscription(local_id).expect("checked existence")
        } else {
            self.insert(request, server_id, channel_size)
        }
    }

    /// De-alias an alias, getting the original ID.
    pub(crate) fn local_id_for(&self, server_id: &SubId) -> Option<B256> {
        self.server_to_local.get(server_id).copied().or_else(|| {
            server_id
                .as_number()
                .and_then(|number| self.server_to_local.get(&SubId::Number(number)).copied())
        })
    }

    /// De-alias an alias, getting the original ID.
    pub(crate) fn server_id_for(&self, local_id: &B256) -> Option<&SubId> {
        self.local_to_server.get_by_left(local_id)
    }

    /// Drop all server_ids.
    pub(crate) fn drop_server_ids(&mut self) {
        self.local_to_server.clear();
        self.server_to_local.clear();
    }

    /// Change the server_id of a subscription.
    fn change_server_id(&mut self, local_id: B256, server_id: SubId) {
        self.remove_server_id(&local_id);
        self.insert_server_id(local_id, server_id);
    }

    /// Remove a subscription by its local_id.
    pub(crate) fn remove_sub(&mut self, local_id: B256) {
        let _ = self.local_to_sub.remove_by_left(&local_id);
        self.remove_server_id(&local_id);
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

    /// Insert the current server ID and any numeric-compatible aliases.
    fn insert_server_id(&mut self, local_id: B256, server_id: SubId) {
        self.insert_server_aliases(local_id, &server_id);
        self.local_to_server.insert(local_id, server_id);
    }

    /// Remove the current server ID and any numeric-compatible aliases.
    fn remove_server_id(&mut self, local_id: &B256) {
        if let Some((local_id, server_id)) = self.local_to_server.remove_by_left(local_id) {
            self.remove_server_aliases(local_id, &server_id);
        }
    }

    fn insert_server_aliases(&mut self, local_id: B256, server_id: &SubId) {
        self.server_to_local.insert(server_id.clone(), local_id);
        if let Some(number) = server_id.as_number() {
            self.server_to_local.insert(SubId::Number(number), local_id);
        }
    }

    fn remove_server_aliases(&mut self, local_id: B256, server_id: &SubId) {
        self.remove_server_alias(local_id, server_id);
        if let Some(number) = server_id.as_number() {
            self.remove_server_alias(local_id, &SubId::Number(number));
        }
    }

    fn remove_server_alias(&mut self, local_id: B256, server_id: &SubId) {
        if self.server_to_local.get(server_id) == Some(&local_id) {
            self.server_to_local.remove(server_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_json_rpc::{Id, Request};
    use alloy_transport::utils::to_json_raw_value;

    #[test]
    fn notifies_subscription_when_server_id_forms_differ() {
        let mut manager = SubscriptionManager::default();
        let request =
            Request::new("eth_subscribe", Id::Number(1), ("newHeads",)).serialize().unwrap();
        let server_id = "0x7413bf1aeb8f1c0087c36b4243f7a41a";

        let mut sub = manager.upsert(request, SubId::String(server_id.to_string()), 16);
        let notification = EthNotification {
            subscription: serde_json::from_str(&format!(r#""{server_id}""#)).unwrap(),
            result: to_json_raw_value(&42).unwrap(),
        };
        assert!(matches!(notification.subscription, SubId::Number(_)));

        manager.notify(notification);

        assert_eq!(sub.try_recv().unwrap().get(), "42");
    }
}
