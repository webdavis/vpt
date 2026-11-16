#![cfg(feature = "dev-tools")]
#[path = "self_test/calendar.rs"]
mod calendar;
#[path = "self_test/engines.rs"]
mod engines;
#[path = "self_test/fixture.rs"]
mod fixture;
#[path = "self_test/named.rs"]
mod named;
#[allow(
    dead_code,
    reason = "shared integration support serves other test targets"
)]
#[path = "show_list_storage/fixture.rs"]
mod observations_fixture;
#[path = "self_test/speakers.rs"]
mod speakers;
#[path = "self_test/summary.rs"]
mod summary;
#[allow(
    dead_code,
    reason = "shared integration support serves other test targets"
)]
mod support;
