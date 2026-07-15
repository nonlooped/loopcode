//! Single integration-test harness. One binary means `cargo test` links the
//! full app library once instead of once per slice file.
//! Run one slice with `cargo test --test integration <module>`, e.g.
//! `cargo test --test integration security_slice3`.

mod support;

mod a11y_slice10;
mod agent_loop_integration;
mod cockpit_slice6;
mod extensibility_slice8;
mod persistence_slice1;
mod providers_slice5;
mod reliability_slice9;
mod runtime_slice2;
mod scaffold_smoke;
mod security_slice3;
mod surfaces_slice7;
mod tools_slice4;
