//! NetPilot Core process entry (NP-013).
//! Full IPC listen loop arrives in later S1 tasks.

use netpilot_core_lib::CoreRuntime;

fn main() {
    let mut runtime = CoreRuntime::new();
    eprintln!(
        "netpilot-core state={}",
        runtime.state().as_str()
    );

    if let Err(err) = runtime.begin_start() {
        eprintln!("start failed: {err}");
        std::process::exit(1);
    }

    // Later: boot IPC, load config, mark_running after readiness.
    if let Err(err) = runtime.mark_running() {
        eprintln!("ready failed: {err}");
        let _ = runtime.mark_failed();
        std::process::exit(1);
    }

    eprintln!(
        "netpilot-core state={}",
        runtime.state().as_str()
    );

    // Skeleton exit path — production will block on IPC / cancellation.
    let _ = runtime.begin_stop();
    let _ = runtime.mark_stopped();
}
