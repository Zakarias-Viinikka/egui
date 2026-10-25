
use super::play::{AppliedStep, Chain};
use super::step::Verb;

/// Total size of the clipboard sentence, in bytes. Very long chains make
/// very long sentences; past this the per-step enumeration is cut short
/// and ends with a tail naming what was left out.
///
/// Derived from a realistic 40-step chain (see the forty_step_sizes
/// test): typical ~5.4 KB, worst case ~7.5 KB, so the cap is the next
/// 1000 above the worst case. About 2k tokens in a chat.
const REPORT_MAX_BYTES: usize = 8000;
/// Bytes set aside for the "(+ N applied step(s) not listed)" tail.
const TAIL_RESERVE: usize = 40;
/// The step section always gets at least this much room, even when the
/// header alone is bigger than the cap, so a huge failure reason can't
/// starve the per-step clauses entirely.
const MIN_STEP_BUDGET: usize = 1500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepStatus {
    Applied,
    Denied,
    Failed,
    Pending,
    Marked,
}

pub fn build(chain: &Chain, statuses: &[StepStatus], failure: Option<&str>) -> String {
    build_capped(chain, statuses, failure, REPORT_MAX_BYTES)
}

/// The full build with the cap as a parameter, so tests can use small
/// caps without touching the real one.
///
/// The header (verdict plus failure reason) is written in full. The
/// step section then gets `cap - header - TAIL_RESERVE`, or
/// `MIN_STEP_BUDGET` when that would be smaller, so a huge header can't
/// starve it.
///
/// Clauses are chosen in two classes. Class one is Denied and Marked;
/// class two is Applied. Within a class, clauses are taken in step order
/// and added whole while they fit; the first that does not fit stops
/// that class, so there are no gaps. Class one is filled before class
/// two. The chosen clauses are then written in step order, so the
/// sentence reads step 1 to step n. The tail names exactly what was
/// left out, per class.
fn build_capped(
    chain: &Chain,
    statuses: &[StepStatus],
    failure: Option<&str>,
    cap: usize,
) -> String {
    let total = chain.steps.len();
    let applied = statuses.iter().filter(|s| **s == StepStatus::Applied).count();
    let denied = statuses.iter().filter(|s| **s == StepStatus::Denied).count();
    let marked = statuses.iter().filter(|s| **s == StepStatus::Marked).count();

    let mut out = String::from("zap: ");
    if let Some(f) = failure {
        out.push_str(&format!("nothing was written. {}.", f));
    } else if total > 0 && applied == total {
        out.push_str(&format!("applied {} step(s).", total));
    } else if total > 0 && denied == total {
        out.push_str(&format!("denied all {} step(s). retry is possible.", total));
    } else {
        out.push_str(&format!(
            "applied {} of {} step(s), denied {}, marked {} for retry.",
            applied, total, denied, marked
        ));
    }

    let header_bytes = out.len();
    let step_budget = cap
        .saturating_sub(header_bytes)
        .saturating_sub(TAIL_RESERVE)
        .max(MIN_STEP_BUDGET);

    // Collect candidates per class.
    let mut class_one: Vec<(usize, String)> = Vec::new();
    let mut class_two: Vec<(usize, String)> = Vec::new();
    for (i, applied_step) in chain.steps.iter().enumerate() {
        let status = statuses.get(i).copied().unwrap_or(StepStatus::Pending);
        let clause = match status {
            StepStatus::Applied => Some(describe(applied_step, false)),
            StepStatus::Denied => Some(describe(applied_step, true)),
            StepStatus::Marked => Some(format!(
                "step {} of {} ({} {}) marked for retry",
                i + 1,
                total,
                verb_of(applied_step),
                applied_step.path.display()
            )),
            _ => None,
        };
        let Some(clause) = clause else { continue };
        match status {
            StepStatus::Applied => class_two.push((i, clause)),
            StepStatus::Denied | StepStatus::Marked => class_one.push((i, clause)),
            _ => {}
        }
    }

    // Pick clauses from class one first, then class two. No gaps in a
    // class: the first clause that does not fit stops that class.
    let mut chosen: Vec<(usize, String)> = Vec::new();
    let mut used = 0usize;

    let mut class_one_cut = 0usize;
    for (i, clause) in &class_one {
        let cost = clause.len() + 2;
        if used + cost > step_budget {
            class_one_cut = class_one.len() - chosen.len();
            break;
        }
        used += cost;
        chosen.push((*i, clause.clone()));
    }

    let class_two_start = chosen.len();
    for (i, clause) in &class_two {
        let cost = clause.len() + 2;
        if used + cost > step_budget {
            break;
        }
        used += cost;
        chosen.push((*i, clause.clone()));
    }
    let class_two_cut = class_two.len() - (chosen.len() - class_two_start);

    chosen.sort_by_key(|(i, _)| *i);
    let any_emitted = !chosen.is_empty();
    for (_, clause) in &chosen {
        out.push(' ');
        out.push_str(clause);
        out.push('.');
    }

    // Overrides (step 12) are uncounted; a chain rarely has more than one
    // and the text is short.
    for o in &chain.overrides {
        if statuses.get(o.step).copied() != Some(StepStatus::Applied) {
            continue;
        }
        out.push(' ');
        out.push_str(&format!(
            "step {} of {}: the reviewer changed the target from {} to {}, and {} later step(s) were updated to match.",
            o.step + 1,
            total,
            o.from,
            o.to,
            o.followers
        ));
    }

    if class_one_cut > 0 || class_two_cut > 0 {
        let mut parts: Vec<String> = Vec::new();
        if class_one_cut > 0 {
            parts.push(format!(
                "+ {class_one_cut} denied/marked step(s) not listed"
            ));
        }
        if class_two_cut > 0 {
            parts.push(format!("+ {class_two_cut} applied step(s) not listed"));
        }
        if any_emitted {
            out.push(' ');
        }
        out.push('(');
        out.push_str(&parts.join(", "));
        out.push(')');
    }

    out
}

fn describe(applied: &AppliedStep, would_have: bool) -> String {
    let prefix = if would_have { "would have " } else { "" };
    let path = applied.path.display();
    if applied.no_op {
        return format!("{}skipped {} (no change)", prefix, path);
    }
    match applied.step.verb {
        Verb::Create => {
            let lines = applied.after.lines().count();
            let first = applied.after.lines().next().unwrap_or("");
            format!(
                "{}created {} ({} lines, starting `{}`)",
                prefix, path, lines, truncate(first, 60)
            )
        }
        Verb::Overwrite => format!("{}overwrote {}", prefix, path),
        Verb::Append => format!("{}appended to {}", prefix, path),
        Verb::Replace => format!(
            "{}replaced `{}` with `{}` in {}",
            prefix,
            truncate(&applied.step.old_text, 60),
            truncate(&applied.step.new_text, 60),
            path
        ),
        Verb::Remove => format!("{}deleted {}", prefix, path),
        Verb::Move => {
            let dest = applied
                .dest
                .as_ref()
                .map(|d| d.display().to_string())
                .unwrap_or_default();
            format!("{}moved {} to {}", prefix, path, dest)
        }
        Verb::Rename => {
            let dest = applied
                .dest
                .as_ref()
                .map(|d| d.display().to_string())
                .unwrap_or_default();
            format!("{}renamed {} to {}", prefix, path, dest)
        }
        Verb::AstGrep => {
            let n = applied.files.len();
            let occurrences: usize = applied.files.iter().map(|f| f.occurrences).sum();
            let mut list: Vec<String> = applied
                .files
                .iter()
                .take(8)
                .map(|f| {
                    f.path
                        .strip_prefix(&applied.path)
                        .unwrap_or(&f.path)
                        .display()
                        .to_string()
                })
                .collect();
            if n > 8 {
                list.push(format!("+{} more", n - 8));
            }
            format!(
                "{}renamed identifier `{}` to `{}` ({} occurrences in {} files under {}: {})",
                prefix,
                applied.step.old_text,
                applied.step.new_text,
                occurrences,
                n,
                path,
                list.join(", ")
            )
        }
        Verb::Mkdir => format!("{}created folder {}", prefix, path),
        Verb::RemoveDir => format!("{}removed folder {}", prefix, path),
        Verb::Cargo => {
            let args = applied.step.args.join(" ");
            let cwd = applied.step.cwd.as_deref().unwrap_or("");
            if would_have {
                format!("would have run cargo {} in {}", args, cwd)
            } else {
                format!("will run cargo {} in {}", args, cwd)
            }
        }
        Verb::Run => {
            let file = applied.step.args.first().cloned().unwrap_or_default();
            let cwd = applied.step.cwd.as_deref().unwrap_or("");
            if would_have {
                format!("would have run {} in {}", file, cwd)
            } else {
                format!("will run {} in {}", file, cwd)
            }
        }
        Verb::Chmod => {
            if would_have {
                format!("would have made {} executable", path)
            } else {
                format!("made {} executable", path)
            }
        }
    }
}

fn verb_of(applied: &AppliedStep) -> &'static str {
    match applied.step.verb {
        Verb::Create => "create",
        Verb::Replace => "replace",
        Verb::Overwrite => "overwrite",
        Verb::Append => "append",
        Verb::Remove => "zaprm",
        Verb::Move => "zapmv",
        Verb::Mkdir => "mkdir",
        Verb::RemoveDir => "rmdir",
        Verb::Cargo => "cargo",
        Verb::Run => "run",
        Verb::Chmod => "chmod",
        Verb::Rename => "rename",
        Verb::AstGrep => "astgrep",
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    let first_line = s.lines().next().unwrap_or("");
    if first_line.chars().count() <= max {
        first_line.to_string()
    } else {
        let mut out: String = first_line.chars().take(max).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::play::play_forward;
    use crate::chain::step::{Step, Verb};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn tmp() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("zap_report_test_{}_{}", std::process::id(), nanos));
        fs::create_dir_all(&p).expect("mkdir");
        p
    }

    fn replace_step(path: &Path, new: &str) -> Step {
        Step {
            verb: Verb::Replace,
            path: path.display().to_string(),
            dest: None,
            old_text: "old".to_string(),
            new_text: new.to_string(),
            count: None,
            no_newline: false,
            args: Vec::new(),
            cwd: None,
        }
    }

    fn small_chain(n: usize) -> (PathBuf, Chain) {
        let dir = tmp();
        let mut steps = Vec::new();
        for i in 0..n {
            let p = dir.join(format!("f{i:02}.rs"));
            fs::write(&p, "old\n").expect("write");
            steps.push(replace_step(&p, "new"));
        }
        let chain = play_forward(steps, &dir).expect("play");
        (dir, chain)
    }

    #[test]
    fn under_cap_all_clauses_present_and_no_tail() {
        let (dir, chain) = small_chain(3);
        let out = build(&chain, &[StepStatus::Applied; 3], None);
        assert!(!out.contains("not listed"), "got: {out}");
        for i in 0..3 {
            assert!(out.contains(&format!("step {} of 3", i + 1)), "got: {out}");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn over_cap_lists_a_prefix_and_ends_with_a_tail() {
        let (dir, chain) = small_chain(20);
        let statuses = vec![StepStatus::Applied; 20];
        let out = build_capped(&chain, &statuses, None, 3000);
        assert!(out.len() <= 3000, "got {} bytes: {out}", out.len());
        assert!(out.contains("applied step(s) not listed"), "got: {out}");
        // First clause present.
        assert!(out.contains("step 1 of 20"), "got: {out}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn marked_and_denied_are_listed_even_when_applied_are_cut() {
        let (dir, chain) = small_chain(40);
        let mut statuses = vec![StepStatus::Applied; 40];
        statuses[32] = StepStatus::Marked; // step 33
        statuses[37] = StepStatus::Marked; // step 38
        statuses[39] = StepStatus::Marked; // step 40
        let out = build_capped(&chain, &statuses, None, 3000);
        assert!(out.contains("step 33 of 40"), "got: {out}");
        assert!(out.contains("step 38 of 40"), "got: {out}");
        assert!(out.contains("step 40 of 40"), "got: {out}");
        assert!(out.contains("applied step(s) not listed"), "got: {out}");
        assert!(!out.contains("denied/marked step(s) not listed"), "got: {out}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn swedish_paths_are_capped_in_bytes() {
        let dir = tmp();
        let mut steps = Vec::new();
        for i in 0..20 {
            let p = dir.join(format!("å_ä_ö_räksmörgås_på_en_ö_{i:02}.rs"));
            fs::write(&p, "old\n").expect("write");
            steps.push(replace_step(&p, "new"));
        }
        let chain = play_forward(steps, &dir).expect("play");
        let statuses = vec![StepStatus::Applied; 20];
        let out = build_capped(&chain, &statuses, None, 3000);
        assert!(out.len() <= 3000, "got {} bytes: {out}", out.len());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn huge_failure_reason_keeps_header_and_gives_steps_room() {
        let (dir, chain) = small_chain(5);
        let reason = "x".repeat(10_000);
        let out = build(&chain, &[StepStatus::Applied; 5], Some(&reason));
        assert!(out.contains(&reason), "header must be intact");
        assert!(out.contains("step 1 of 5"), "got: {out}");
        // The header alone (10 KB) plus the min step budget.
        assert!(out.len() >= 10_000 + 1_500, "got {} bytes", out.len());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn forty_step_sizes() {
        // A realistic 40-step chain: 15 replace, 12 create, 6 overwrite,
        // 3 move, 2 rmdir, 2 append. Prints three sizes so the plan's
        // arithmetic can be checked; see the REPORT_MAX_BYTES comment.
        let dir = tmp();
        for sub in ["src", "components", "widgets"] {
            fs::create_dir_all(dir.join(sub)).expect("mkdir");
        }
        let rel = |name: &str| -> PathBuf {
            dir.join("src").join("components").join("widgets").join(name)
        };
        let mut steps = Vec::new();
        for i in 0..15 {
            let p = rel(&format!("replace_target_{i:02}.rs"));
            fs::write(&p, "fn target() {}\n").expect("write");
            steps.push(Step {
                verb: Verb::Replace,
                path: p.display().to_string(),
                dest: None,
                old_text: "fn target() {}".to_string(),
                new_text: "fn target() { }".to_string(),
                count: None,
                no_newline: false,
                args: Vec::new(),
                cwd: None,
            });
        }
        for i in 0..12 {
            let p = rel(&format!("new_file_{i:02}.rs"));
            steps.push(Step {
                verb: Verb::Create,
                path: p.display().to_string(),
                dest: None,
                old_text: String::new(),
                new_text: format!("// created {i}\nfn x() {{}}\n"),
                count: None,
                no_newline: false,
                args: Vec::new(),
                cwd: None,
            });
        }
        for i in 0..6 {
            let p = rel(&format!("overwrite_{i:02}.rs"));
            fs::write(&p, "old\n").expect("write");
            steps.push(Step {
                verb: Verb::Overwrite,
                path: p.display().to_string(),
                dest: None,
                old_text: String::new(),
                new_text: format!("// overwritten {i}\n"),
                count: None,
                no_newline: false,
                args: Vec::new(),
                cwd: None,
            });
        }
        for i in 0..3 {
            let from = rel(&format!("move_from_{i:02}.rs"));
            let to = rel(&format!("move_to_{i:02}.rs"));
            fs::write(&from, "content\n").expect("write");
            steps.push(Step {
                verb: Verb::Move,
                path: from.display().to_string(),
                dest: Some(to.display().to_string()),
                old_text: String::new(),
                new_text: String::new(),
                count: None,
                no_newline: false,
                args: Vec::new(),
                cwd: None,
            });
        }
        for i in 0..2 {
            let p = dir.join(format!("empty_dir_{i:02}"));
            fs::create_dir_all(&p).expect("mkdir");
            steps.push(Step {
                verb: Verb::RemoveDir,
                path: p.display().to_string(),
                dest: None,
                old_text: String::new(),
                new_text: String::new(),
                count: None,
                no_newline: false,
                args: Vec::new(),
                cwd: None,
            });
        }
        for i in 0..2 {
            let p = rel(&format!("append_{i:02}.rs"));
            fs::write(&p, "start\n").expect("write");
            steps.push(Step {
                verb: Verb::Append,
                path: p.display().to_string(),
                dest: None,
                old_text: String::new(),
                new_text: "// appended\n".to_string(),
                count: None,
                no_newline: false,
                args: Vec::new(),
                cwd: None,
            });
        }
        let chain = play_forward(steps, &dir).expect("play");
        assert_eq!(chain.steps.len(), 40);

        let typical = build(&chain, &vec![StepStatus::Applied; 40], None);
        let denied = build(&chain, &vec![StepStatus::Denied; 40], None);

        eprintln!("typical 40-step report: {} bytes", typical.len());
        eprintln!("all-denied 40-step report: {} bytes", denied.len());
        eprintln!("cap: {} bytes", REPORT_MAX_BYTES);

        assert!(
            typical.len() <= REPORT_MAX_BYTES,
            "typical {} exceeds cap {}",
            typical.len(),
            REPORT_MAX_BYTES
        );
        assert!(
            denied.len() <= REPORT_MAX_BYTES,
            "denied {} exceeds cap {}",
            denied.len(),
            REPORT_MAX_BYTES
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
