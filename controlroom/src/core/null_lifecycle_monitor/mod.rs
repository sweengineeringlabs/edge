//! `NullLifecycleMonitor` — no-op `LifecycleMonitor` for tests and early
//! bring-up.

mod monitor;

pub(crate) use monitor::NullLifecycleMonitor;
