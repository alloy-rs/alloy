use crate::managers::ActiveSubscription;
use alloy_json_rpc::{EthNotification, SubId};
use alloy_primitives::B256;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub(crate) struct SubscriptionManager {
    /// Active subscriptions keyed by their local ID.
    subscriptions: BTreeMap<B256, ActiveSubscription>,
    /// Current server IDs to local IDs.
    servers: BTreeMap<SubId, B256>,
}

impl SubscriptionManager {
    /// Get an iterator over the subscriptions.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&B256, &ActiveSubscription)> {
        self.subscriptions.iter()
    }

    /// Get the number of subscriptions.
    pub(crate) fn len(&self) -> usize {
        self.subscriptions.len()
    }

    /// Insert a subscription.
    pub(crate) fn insert(&mut self, active: ActiveSubscription, server_id: SubId) {
        let local_id = active.local_id;
        let previous_server = self.servers.insert(server_id, local_id);
        debug_assert!(previous_server.is_none(), "server subscription id must not be overwritten");
        let previous = self.subscriptions.insert(local_id, active);
        debug_assert!(previous.is_none(), "active subscription must not be overwritten");
    }

    /// Get the local ID associated with a server ID.
    pub(crate) fn local_id_for(&self, server_id: &SubId) -> Option<B256> {
        self.servers.get(server_id).copied()
    }

    /// Drop all server_ids.
    pub(crate) fn drop_server_ids(&mut self) {
        self.servers.clear();
    }

    /// Associate a new server id with an existing subscription without overwriting another id.
    pub(crate) fn set_server_id(&mut self, local_id: &B256, server_id: SubId) -> bool {
        if !self.subscriptions.contains_key(local_id)
            || self.server_id_for_local_id(local_id).is_some()
            || self.servers.contains_key(&server_id)
        {
            return false;
        }
        self.servers.insert(server_id, *local_id);
        true
    }

    /// Remove a subscription by its local_id.
    pub(crate) fn remove_sub(
        &mut self,
        local_id: B256,
    ) -> Option<(ActiveSubscription, Option<SubId>)> {
        let subscription = self.subscriptions.remove(&local_id)?;
        let server_id = self.server_id_for_local_id(&local_id).cloned();
        if let Some(server_id) = &server_id {
            self.servers.remove(server_id);
        }
        Some((subscription, server_id))
    }

    /// Notify the subscription channel of a new value, if the sub is known,
    /// and if any receiver exists. If the sub id is unknown, or no receiver
    /// exists, the notification is dropped.
    pub(crate) fn notify(&mut self, notification: EthNotification) {
        if let Some(local_id) = self.local_id_for(&notification.subscription) {
            if let Some(sub) = self.get(&local_id) {
                sub.notify(notification.result);
            }
        }
    }

    pub(crate) fn get(&self, local_id: &B256) -> Option<&ActiveSubscription> {
        self.subscriptions.get(local_id)
    }

    pub(crate) fn get_mut(&mut self, local_id: &B256) -> Option<&mut ActiveSubscription> {
        self.subscriptions.get_mut(local_id)
    }

    pub(crate) fn contains_server_id(&self, server_id: &SubId) -> bool {
        self.servers.contains_key(server_id)
    }

    pub(crate) fn server_id_for_local_id(&self, local_id: &B256) -> Option<&SubId> {
        self.servers
            .iter()
            .find_map(|(server_id, candidate)| (candidate == local_id).then_some(server_id))
    }
}
