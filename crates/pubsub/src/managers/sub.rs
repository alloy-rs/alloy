use crate::{managers::ActiveSubscription, RawSubscription};
use alloy_json_rpc::{EthNotification, Id, SerializedRequest, SubId};
use alloy_primitives::B256;
use bimap::BiBTreeMap;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub(crate) struct SubscriptionManager {
    /// The subscriptions.
    local_to_sub: BTreeMap<B256, ActiveSubscription>,
    /// Tracks the CURRENT server id for a subscription.
    local_to_server: BiBTreeMap<B256, SubId>,
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

    /// Returns true if a subscription with this local_id exists.
    pub(crate) fn contains(&self, local_id: &B256) -> bool {
        self.local_to_sub.contains_key(local_id)
    }

    /// Returns true if the subscription with this local_id is routed via a server id.
    pub(crate) fn is_routed(&self, local_id: &B256) -> bool {
        self.local_to_server.contains_left(local_id)
    }

    /// Insert a new subscription routed via `server_id`, returning its local id.
    pub(crate) fn insert(
        &mut self,
        request: SerializedRequest,
        server_id: SubId,
        channel_size: usize,
    ) -> B256 {
        let active = ActiveSubscription::new(request, channel_size);
        let local_id = active.local_id;
        self.local_to_server.insert(local_id, server_id);
        self.local_to_sub.insert(local_id, active);
        local_id
    }

    /// Route an existing subscription via `server_id`, unless it is already routed via another
    /// server id.
    ///
    /// The current server id is never overwritten: that would orphan the server-side
    /// subscription behind it. Instead, the rejected `server_id` is returned so that the caller can
    /// cancel it.
    pub(crate) fn set_server_id(&mut self, local_id: B256, server_id: SubId) -> Result<(), SubId> {
        match self.local_to_server.get_by_left(&local_id) {
            Some(current) if *current == server_id => Ok(()),
            Some(_) => Err(server_id),
            None => {
                self.local_to_server.insert(local_id, server_id);
                Ok(())
            }
        }
    }

    /// Record that a caller was handed the local id and is expected to fetch a receiver, and to
    /// unsubscribe once.
    pub(crate) fn add_pending_claim(&mut self, local_id: B256) {
        if let Some(sub) = self.local_to_sub.get_mut(&local_id) {
            sub.pending_claims += 1;
            sub.shares += 1;
        }
    }

    /// De-alias an alias, getting the original ID.
    pub(crate) fn local_id_for(&self, server_id: &SubId) -> Option<B256> {
        self.local_to_server.get_by_right(server_id).copied()
    }

    /// Drop all server_ids.
    pub(crate) fn drop_server_ids(&mut self) {
        self.local_to_server.clear();
    }

    /// Remove all subscriptions that nobody consumes.
    pub(crate) fn retain_consumed(&mut self) {
        let unconsumed = self
            .local_to_sub
            .iter()
            .filter(|(_, sub)| !sub.has_consumers())
            .map(|(local_id, _)| *local_id)
            .collect::<Vec<_>>();
        for local_id in unconsumed {
            debug!(%local_id, "Dropping subscription without consumers");
            self.remove_sub(local_id);
        }
    }

    /// Count the resubscriptions that the dying backend did not acknowledge, and remove the
    /// subscriptions that more than `max_replays` consecutive backends did not acknowledge. This
    /// closes their receivers.
    pub(crate) fn count_unacked_resubscribes(&mut self, max_replays: u32) {
        let mut exhausted = Vec::new();
        for (local_id, sub) in &mut self.local_to_sub {
            if self.local_to_server.contains_left(local_id) {
                sub.unacked_resubscribes = 0;
            } else {
                sub.unacked_resubscribes += 1;
                if sub.unacked_resubscribes > max_replays {
                    exhausted.push(*local_id);
                }
            }
        }
        for local_id in exhausted {
            warn!(
                %local_id,
                "Backend died {} times before acknowledging the resubscription, dropping the subscription",
                max_replays + 1
            );
            self.remove_sub(local_id);
        }
    }

    /// Remove a subscription by its local_id, returning it and its current server id.
    pub(crate) fn remove_sub(
        &mut self,
        local_id: B256,
    ) -> Option<(ActiveSubscription, Option<SubId>)> {
        let server_id = self.local_to_server.remove_by_left(&local_id).map(|(_, id)| id);
        self.local_to_sub.remove(&local_id).map(|sub| (sub, server_id))
    }

    /// Notify the subscription channel of a new value, if the sub is known,
    /// and if any receiver exists. If the sub id is unknown, or no receiver
    /// exists, the notification is dropped.
    pub(crate) fn notify(&mut self, notification: EthNotification) {
        if let Some(local_id) = self.local_id_for(&notification.subscription) {
            if let Some(sub) = self.local_to_sub.get(&local_id) {
                sub.notify(notification.result);
            }
        }
    }

    /// Get a receiver for a subscription.
    pub(crate) fn get_subscription(&mut self, local_id: B256) -> Option<RawSubscription> {
        let sub = self.local_to_sub.get_mut(&local_id)?;
        sub.pending_claims = sub.pending_claims.saturating_sub(1);
        Some(sub.subscribe())
    }

    /// Release the share of one caller that was handed the local id, and remove the subscription
    /// once no such caller is left, returning it and its current server id.
    ///
    /// Returns `None` if the subscription is unknown or still shared with another caller.
    pub(crate) fn release(
        &mut self,
        local_id: B256,
    ) -> Option<(ActiveSubscription, Option<SubId>)> {
        let sub = self.local_to_sub.get_mut(&local_id)?;
        sub.shares = sub.shares.saturating_sub(1);
        if sub.shares > 0 {
            return None;
        }
        self.remove_sub(local_id)
    }

    /// Returns true if `id` is the id of the request that re-creates the unrouted subscription
    /// with this local_id after a reconnect.
    pub(crate) fn is_resubscription(&self, local_id: &B256, id: &Id) -> bool {
        !self.is_routed(local_id)
            && self.local_to_sub.get(local_id).is_some_and(|sub| sub.request.id() == id)
    }
}
