use std::process::ExitCode;

use symposium_sdk::hook::{HookHandler, run};

struct JustRightRust;

impl HookHandler for JustRightRust {}

fn main() -> ExitCode {
    run(JustRightRust)
}
