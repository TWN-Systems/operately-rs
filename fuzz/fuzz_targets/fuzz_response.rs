//! Fuzz the dominant hostile-input surface in an SDK: the server's response
//! body.
//!
//! `OperatelyClient::decode` ends in `serde_json::from_str::<Out>(&body)` on
//! whatever bytes came back. A panic, an unbounded allocation, or a stack
//! overflow in a `Deserialize` impl is a DoS in the *caller's* process, and the
//! bytes are attacker-influenced whenever the deployment is hostile or the
//! connection is intercepted.
//!
//! Three generated types are driven because they exercise different shapes:
//! `Person` is flat with scalars, `Task` reaches a nested object graph, and
//! `Activity` reaches the union types the generator deliberately leaves as raw
//! `serde_json::Value`. If they behave, the 792 generated structs share the
//! same emission path — see `build.rs`'s generator regression job.

#![no_main]

use libfuzzer_sys::fuzz_target;
use operately_sdk::{Activity, Person, Task};

fuzz_target!(|data: &[u8]| {
    // A rejection is the expected outcome for almost every input. Only the
    // panicking outcomes are findings.
    let _ = serde_json::from_slice::<Person>(data);
    let _ = serde_json::from_slice::<Task>(data);
    let _ = serde_json::from_slice::<Activity>(data);
});
