use crate::{
    managers::{ActiveSubscription, Share},
    RawSubscription,
};
use alloy_json_rpc::{EthNotification, Id, SerializedRequest, SubId};
use alloy_primitives::{map::B256Map, Keccak256, B256};
use bimap::BiBTreeMap;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub(crate) struct SubscriptionManager {
    /// The subscriptions.
    local_to_sub: BTreeMap<B256, ActiveSubscription>,
    /// Tracks the CURRENT server id for a subscription.
    local_to_server: BiBTreeMap<B256, SubId>,
    /// The subscription of each local id handed to a caller.
    alias_to_local: B256Map<B256>,
    /// The number of local ids handed to callers so far.
    aliases_issued: u64,
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
    pub(crate) fn insert(&mut self, request: SerializedRequest, server_id: SubId) -> B256 {
        let active = ActiveSubscription::new(request);
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

    /// Returns a new local id to hand to a caller of the subscription `local_id`.
    ///
    /// Local ids are never reused, so a caller's id never refers to another caller's share, even
    /// after its subscription was dropped and created afresh.
    pub(crate) fn new_alias(&mut self, local_id: B256) -> B256 {
        self.aliases_issued += 1;
        let mut hasher = Keccak256::new();
        hasher.update(local_id);
        hasher.update(self.aliases_issued.to_be_bytes());
        hasher.finalize()
    }

    /// Record that a caller was handed `alias` for the subscription `local_id`, and is expected to
    /// fetch a receiver with it and to unsubscribe it.
    ///
    /// The shares of callers that dropped every receiver without unsubscribing are removed.
    pub(crate) fn add_share(&mut self, local_id: B256, alias: B256, channel_size: usize) {
        let Self { local_to_sub, alias_to_local, .. } = self;
        let Some(sub) = local_to_sub.get_mut(&local_id) else { return };
        sub.shares.retain(|alias, share| {
            let live = share.is_live();
            if !live {
                alias_to_local.remove(alias);
            }
            live
        });
        sub.shares.insert(alias, Share::new(channel_size));
        alias_to_local.insert(alias, local_id);
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
        let sub = self.local_to_sub.remove(&local_id)?;
        for alias in sub.shares.keys() {
            self.alias_to_local.remove(alias);
        }
        Some((sub, server_id))
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

    /// Get a receiver for the subscription that `alias` was handed out for.
    pub(crate) fn get_subscription(&mut self, alias: B256) -> Option<RawSubscription> {
        let local_id = self.alias_to_local.get(&alias)?;
        let share = self.local_to_sub.get_mut(local_id)?.shares.get_mut(&alias)?;
        Some(RawSubscription { rx: share.subscribe(), local_id: alias })
    }

    /// Remove the share of the caller that was handed `alias`, which closes the receivers fetched
    /// with it. Once no caller holds a receiver or has yet to fetch one, the subscription is
    /// removed too, and returned with its current server id.
    ///
    /// Returns `None` if `alias` is unknown, e.g. because its subscription was dropped, or if
    /// another caller still consumes the subscription.
    pub(crate) fn release(&mut self, alias: B256) -> Option<(ActiveSubscription, Option<SubId>)> {
        let local_id = self.alias_to_local.remove(&alias)?;
        let sub = self.local_to_sub.get_mut(&local_id)?;
        sub.shares.remove(&alias);
        if sub.has_consumers() {
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
