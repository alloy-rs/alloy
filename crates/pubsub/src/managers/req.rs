use crate::{managers::InFlight, time::Instant};
use alloy_json_rpc::{Id, Response, SubId};
use alloy_primitives::map::HashMap;
use alloy_transport::TransportErrorKind;
use std::{io, sync::Arc};

/// Manages in-flight requests.
#[derive(Debug, Default)]
pub(crate) struct RequestManager {
    reqs: HashMap<Id, InFlight>,
}

impl RequestManager {
    /// Get the number of in-flight requests.
    pub(crate) fn len(&self) -> usize {
        self.reqs.len()
    }

    /// Get an iterator over the in-flight requests.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&Id, &InFlight)> {
        self.reqs.iter()
    }

    /// Insert a new in-flight request.
    pub(crate) fn insert(&mut self, in_flight: InFlight) {
        self.reqs.insert(in_flight.request.id().clone(), in_flight);
    }

    pub(crate) fn cancel(&mut self, id: &Id, identity: &Arc<()>) {
        if self.reqs.get(id).is_some_and(|request| Arc::ptr_eq(&request.identity, identity)) {
            self.reqs.remove(id);
        }
    }

    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        self.reqs.values().filter_map(|request| request.deadline).min()
    }

    pub(crate) fn expire(&mut self) {
        for (_, request) in self.reqs.extract_if(|_, request| !request.is_live()) {
            // Closing the channel alone would report BackendGone even though
            // the connection manager still serves active subscriptions.
            let _ = request
                .tx
                .send(Err(TransportErrorKind::custom(io::Error::from(io::ErrorKind::TimedOut))));
        }
    }

    pub(crate) fn fail_all(&mut self, detail: &str) {
        for (_, request) in std::mem::take(&mut self.reqs) {
            let _ = request.tx.send(Err(TransportErrorKind::non_retryable_str(detail)));
        }
    }

    pub(crate) fn disconnect(&mut self) {
        // Completed responses have already left this map. The provider alone
        // retries reads; this manager only restores subscriptions.
        let pending = std::mem::take(&mut self.reqs);
        for (id, request) in pending {
            if request.is_subscription() && request.is_live() && !request.subscription_replay {
                self.reqs.insert(id, request);
            } else {
                let _ = request.tx.send(Err(TransportErrorKind::custom(io::Error::from(
                    io::ErrorKind::ConnectionReset,
                ))));
            }
        }
    }

    /// Handle a response by sending the payload to the waiter.
    ///
    /// If the request created a new subscription, this function returns the
    /// subscription ID and the in-flight request for conversion to an
    /// `ActiveSubscription`.
    pub(crate) fn handle_response(&mut self, resp: Response) -> Option<(SubId, InFlight)> {
        if let Some(in_flight) = self.reqs.remove(&resp.id) {
            return in_flight.fulfill(resp);
        }
        None
    }
}
