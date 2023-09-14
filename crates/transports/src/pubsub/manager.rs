use std::collections::BTreeMap;

use alloy_json_rpc::{EthNotification, Id};
use alloy_primitives::U256;
use bimap::BiBTreeMap;

use super::ActiveSubscription;

#[derive(Default, Debug)]
pub struct SubscriptionManager {
    subs: BTreeMap<Id, ActiveSubscription>,
    aliases: BiBTreeMap<U256, Id>,
}

impl SubscriptionManager {
    /// Get a ref to the alias bimap.
    pub fn aliases(&self) -> &BiBTreeMap<U256, Id> {
        &self.aliases
    }

    pub fn drop_aliases(&mut self) {
        self.aliases.clear();
    }

    /// Get a reference to a subscription.
    pub fn sub(&self, id: &Id) -> Option<&ActiveSubscription> {
        self.subs.get(id)
    }

    /// Get a mutable reference to a subscription.
    pub fn sub_mut(&mut self, id: &Id) -> Option<&mut ActiveSubscription> {
        self.subs.get_mut(id)
    }

    /// Get a mutable reference to a subscription by alias.
    pub fn sub_mut_by_alias(&mut self, alias: U256) -> Option<&mut ActiveSubscription> {
        if let Some(id) = self.aliases.get_by_left(&alias) {
            self.subs.get_mut(id)
        } else {
            None
        }
    }

    /// Change the alias of a subscription.
    pub fn change_alias(&mut self, id: &Id, alias: U256) {
        self.aliases.insert(alias, id.clone());
    }

    /// Get sub params.
    pub fn params(&self, id: &Id) -> Option<&serde_json::value::RawValue> {
        self.subs.get(id).map(|sub| sub.params())
    }

    /// Get sub params by alias.
    pub fn params_by_alias(&self, alias: U256) -> Option<&serde_json::value::RawValue> {
        if let Some(id) = self.aliases.get_by_left(&alias) {
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
        if let Some((_, id)) = self.aliases.remove_by_left(&alias) {
            self.subs.remove(&id);
        }
    }

    /// Send a notification. Remove the sub and alias if sending fails.
    pub fn send(&mut self, notification: EthNotification<Box<serde_json::value::RawValue>>) {
        if let Some(sub) = self.sub_mut_by_alias(notification.subscription) {
            if let Err(_) = sub.notify(notification.result) {
                self.remove_sub_and_alias(notification.subscription);
            }
        }
    }
}
