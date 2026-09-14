//! NetPilot Core process entry (NP-014 lifecycle).
//! Full IPC listen loop arrives in later S1 tasks.

use netpilot_core_lib::CoreRuntime;

fn main() {
    let mut runtime = CoreRuntime::new();
    eprintln!("netpilot-core state={}", runtime.state().as_str());

    if let Err(err) = runtime.start() {
        eprintln!("start failed: {err}");
        std::process::exit(1);
    }

    eprintln!("netpilot-core state={}", runtime.state().as_str());

    // Skeleton: no IPC block yet — shut down cleanly.
    if let Err(err) = runtime.shutdown() {
        eprintln!("shutdown failed: {err}");
        std::process::exit(1);
    }

    eprintln!("netpilot-core state={}", runtime.state().as_str());
}
