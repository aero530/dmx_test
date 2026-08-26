//! Executor glue for the event router.
//!
//! The router itself — `Router` and all its routing logic — is target-agnostic
//! and lives in `common::event_router`. Only the embassy task wrapper is here,
//! because `#[embassy_executor::task]` ties it to this crate's executor.

pub use common::event_router::*;

use embassy_futures::select::{select, Either};

#[embassy_executor::task]
pub async fn event_router(mut router: Router) {
    loop {
        // Event-driven: wake on whichever channel has data instead of
        // polling each with a timeout (which added up to ~10ms latency per
        // event and constant wakeups).
        match select(router.channel.receive(), router.channel_dmx.receive()).await {
            Either::First(new_message) => router.process_router_event(new_message).await,
            Either::Second(new_message) => router.process_dmx_event(new_message).await,
        }
    }
}
