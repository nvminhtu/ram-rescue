//! Server Hub core: scan run configs, read live ports/processes, start and stop servers. No UI here.
pub mod api;
pub mod guard;
pub mod health;
pub mod license;
pub mod mem;
pub mod procs;
pub mod scan;
pub mod share;
