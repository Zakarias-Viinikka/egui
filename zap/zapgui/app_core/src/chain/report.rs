
use super::play::{AppliedStep, Chain};
use super::step::Verb;

/// Total size of the clipboard sentence, in bytes. Very long chains make
/// very long sentences; past this the per-step enumeration is cut short
/// and ends with a "(+ N more steps)" tail.
const REPORT_MAX_BYTES: usize = 4000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepStatus {
    Applied,
    Denied,
    Failed,
    Pending,
    Marked,
}

pub fn build(chain: &Chain, statuses: &[StepStatus], failure: Option<&str>) -> String {
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

    let mut emitted = 0usize;
    let mut skipped = 0usize;
    for (i, applied_step) in chain.steps.iter().enumerate() {
        let clause = match statuses.get(i).copied().unwrap_or(StepStatus::Pending) {
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
        let next = clause.chars().count() + 2;
        if out.len() + next > REPORT_MAX_BYTES {
            skipped += 1;
            continue;
        }
        out.push(' ');
        out.push_str(&clause);
        out.push('.');
        emitted += 1;
    }
    if skipped > 0 {
        if emitted > 0 {
            out.push(' ');
        }
        out.push_str(&format!("(+ {skipped} more step(s))"));
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
    }
}

fn truncate(s: &str, max: usize) -> String {
    let first_line = s.lines().next().unwrap_or("");
    if first_line.chars().count() <= max {
        first_line.to_string()
    } else {
        let mut out: String = first_line.chars().take(max).collect();
        out.push('…');
        out
    }
}
