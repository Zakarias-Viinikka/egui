
use super::play::{AppliedStep, Chain};
use super::step::Verb;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepStatus {
    Applied,
    Denied,
    Failed,
    Pending,
}

pub fn build(chain: &Chain, statuses: &[StepStatus], failure: Option<&str>) -> String {
    let total = chain.steps.len();
    let applied = statuses.iter().filter(|s| **s == StepStatus::Applied).count();
    let denied = statuses.iter().filter(|s| **s == StepStatus::Denied).count();

    let mut out = String::from("zap: ");
    if let Some(f) = failure {
        out.push_str(&format!("nothing was written. {}.", f));
    } else if total > 0 && applied == total {
        out.push_str(&format!("applied {} step(s).", total));
    } else if total > 0 && denied == total {
        out.push_str(&format!("denied all {} step(s).", total));
    } else {
        out.push_str(&format!("applied {} of {} step(s), denied {}.", applied, total, denied));
    }

    for (i, applied_step) in chain.steps.iter().enumerate() {
        match statuses.get(i).copied().unwrap_or(StepStatus::Pending) {
            StepStatus::Applied => {
                out.push(' ');
                out.push_str(&describe(applied_step, false));
                out.push('.');
            }
            StepStatus::Denied => {
                out.push(' ');
                out.push_str(&describe(applied_step, true));
                out.push('.');
            }
            _ => {}
        }
    }

    out
}

fn describe(applied: &AppliedStep, would_have: bool) -> String {
    let prefix = if would_have { "would have " } else { "" };
    let path = applied.path.display();
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
