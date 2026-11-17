//! Parallel scenario runs never collide on ports.
//!
//! Its own test binary: cargo runs test binaries one after another, so its engines never
//! share the machine with the engines of the other runner tests.
#![cfg(test)]

mod common;

use std::path::PathBuf;

use common::{real_host, workspace_root};
use packie_sim::report::Outcome;

/// Scenarios that bind engine ports and `tcp_server` ports, fast and without capabilities.
const COLLISION_SUITE: [&str; 8] = [
    "bridge-close-propagation",
    "bridge-tap-hl7",
    "bridge-tcp-full-duplex",
    "bridge-pause-resume",
    "config-apply-per-channel",
    "msg-duplicate",
    "msg-hl7-in-ack",
    "msg-resend",
];

/// Scenarios run at once. Windows runners start processes and commit to SQLite slowly
/// enough that eight engines on four cores push single steps past their 5 s waits; four
/// at once is what the scenario suite runs with, and where the port race appeared.
const AT_ONCE: usize = if cfg!(windows) { 4 } else { 8 };

/// The regression test of the parallel-run flake. Scenarios used to probe a free port, release
/// it and bind it later, and a neighbour's outgoing connection could take it in between
/// ("bind 127.0.0.1:<port>: Address already in use", about one full run in two). The eight
/// scenarios now run [`AT_ONCE`] at a time, twenty times over, which is 160 runs and over a
/// thousand reserved ports; any collision fails the test and names the scenario.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn parallel_scenarios_twenty_times_never_collide_on_ports() {
    let files: Vec<PathBuf> = COLLISION_SUITE
        .iter()
        .map(|name| workspace_root().join(format!("tests/scenarios/{name}.json")))
        .collect();
    assert!(
        files.iter().all(|f| f.is_file()),
        "the collision suite exists"
    );
    let host = real_host().await;
    for round in 0..20 {
        let results = packie_sim::runner::run_many(&files, &host, AT_ONCE, None).await;
        for result in results {
            let report = result.unwrap();
            let text = report.human(true);
            assert!(
                !text.contains("Address already in use"),
                "round {round}: port collision\n{text}"
            );
            assert_eq!(
                report.outcome,
                Outcome::Passed,
                "round {round}: {}",
                report.human(true)
            );
            assert!(report.kept_dir.is_none(), "round {round}: {text}");
        }
    }
}
