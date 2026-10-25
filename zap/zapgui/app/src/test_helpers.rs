//! Test-only helpers shared by the app's unit tests. Not linked into the
//! binary unless tests are running.

use app_core::chain::{play_forward, Step, Verb};
use std::path::PathBuf;

/// Build a Chain with `n` no-op steps on `/tmp/dummy_{i}.rs`. No file is
/// touched; the writer is never called in these tests.
pub fn dummy_chain(n: usize) -> app_core::chain::Chain {
    let mut steps = Vec::with_capacity(n);
    for i in 0..n {
        steps.push(Step {
            verb: Verb::Replace,
            path: format!("/tmp/dummy_{i}.rs"),
            dest: None,
            old_text: "x".to_string(),
            new_text: "y".to_string(),
            count: None,
            no_newline: false,
            args: Vec::new(),
            cwd: None,
        });
    }
    // Play against a cwd that does not need to exist; replace fails on a
    // missing file, so seed the files map by hand is not an option here.
    // Instead, run play_forward against the real /tmp after creating the
    // files. Cleanup is the caller's responsibility.
    for i in 0..n {
        let p = PathBuf::from(format!("/tmp/dummy_{i}.rs"));
        let _ = std::fs::write(&p, "x\n");
    }
    play_forward(steps, std::path::Path::new("/"))
        .expect("dummy chain should play")
}
