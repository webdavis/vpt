#![cfg(feature = "dev-tools")]
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
#[allow(
    dead_code,
    reason = "shared integration support serves other test targets"
)]
mod support;
