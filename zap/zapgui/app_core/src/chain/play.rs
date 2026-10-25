use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::Sender;

use crate::text_diff::moves as move_detect;
use crate::text_diff::{
    DiffRow, DiffRowProcessedForUi, MoveSpan, RowKind, Side, pair_lines, pair_lines_full, process,
};

use crate::text_diff::similarity::{
    ClosestLine, EDIT_THRESHOLD, SIMILARITY_THRESHOLD, closest_line, edit_distance,
    inline_spans, similarity as line_similarity, tokenize,
};

use super::astgrep;
use super::report::truncate;
use super::step::{Step, Verb};

/// rm refuses to keep an undo copy of a file larger than this.
const RM_MAX_BYTES: u64 = 1024 * 1024;

pub struct AppliedStep {
    pub step: Step,
    pub path: PathBuf,
    /// Move only: the destination, resolved against the chain's cwd.
    pub dest: Option<PathBuf>,
    pub before: Option<String>,
    pub after: String,
    pub rows: Vec<DiffRowProcessedForUi>,
    /// Move spans detected across the whole chain. Empty until move
    /// detection lands.
    pub move_spans: Vec<MoveSpan>,
    /// astgrep only: the files the rename changes, sorted by path. Empty
    /// for every other verb. `path` is the folder, and `before` and
    /// `after` hold these files joined in the same order, each preceded
    /// by a line naming it.
    pub files: Vec<astgrep::FileEdit>,
    /// True when the step was a no-op: overwrite with identical content,
    /// replace where old == new, append with empty text. The writer skips
    /// these and the report describes them as "skipped".
    pub no_op: bool,
}

pub struct Snapshot {
    pub path: PathBuf,
    pub content: Option<String>,
}

/// One edited target, recorded so the report can tell the AI what
/// changed. `step` is 0-based.
#[derive(Clone, Debug)]
pub struct Override {
    pub step: usize,
    pub from: String,
    pub to: String,
    pub followers: usize,
}

pub struct Chain {
    pub steps: Vec<AppliedStep>,
    pub snapshots: Vec<Snapshot>,
    /// Set only for chains that arrive over a ChainRun frame. A streaming
    /// handler puts a sender here and waits for the outcome. Wrapped in a
    /// Mutex because mpsc::Sender is Send but not Sync, and Chain is
    /// stored inside an Arc.
    pub notify: Option<Mutex<Sender<ChainOutcome>>>,
    /// Deepest common folder of every path the chain touches. For a
    /// single-file chain this is the file's own parent. Used by the
    /// review window to say "edits confined to <this>".
    pub boundary: PathBuf,
    /// Steps that need eyes. Empty means the review shows every step (the
    /// default for play_forward and for retry). A non-empty list is the
    /// set of step indices that are shown; the rest are written with the
    /// ones that are. Set by handle_chain and friends after
    /// steps_that_need_eyes runs.
    pub review_indices: Vec<usize>,
    /// Targets the reviewer changed with the pen, oldest first.
    pub overrides: Vec<Override>,
}

/// An Arc<Chain> in a shape that can derive Debug without Chain having
/// to. Only the step count is shown.
#[derive(Clone)]
pub struct SupersededChain(pub Arc<Chain>);

impl std::fmt::Debug for SupersededChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SupersededChain({} steps)", self.0.steps.len())
    }
}

#[derive(Clone, Debug)]
pub enum ChainOutcome {
    /// Some steps applied, some denied. The booleans match the steps:
    /// true means applied.
    Applied(Vec<bool>),
    /// Every step denied. Nothing was written.
    AllDenied,
    /// A newer chain replaced this one before the user acted.
    Replaced,
    /// The reviewer edited a step's target with the pen and the chain
    /// was re-planned. The wait loop reassigns `mine` to the new Arc.
    Superseded(SupersededChain),
}

impl Chain {
    /// True if any step needs review even when every file is whitelisted.
    /// A run step joins the rm/mv rule because the run is a side effect
    /// that has to be confirmed.
    pub fn needs_review(&self) -> bool {
        self.steps.iter().any(|s| s.step.verb.always_reviewed())
    }

    /// Send the outcome to the waiting handler, if one is there.
    pub fn finish(&self, outcome: ChainOutcome) {
        if let Some(m) = &self.notify {
            if let Ok(tx) = m.lock() {
                let _ = tx.send(outcome);
            }
        }
    }

    /// The path and after text of the last content-writing step per file,
    /// honoring `wanted`. A missing entry in `wanted` counts as true, so
    /// an empty slice means every step. Steps that write no content
    /// (zaprm, zapmv, mkdir, rmdir, cargo, run) never appear.
    pub fn final_writes(&self, wanted: &[bool]) -> Vec<(PathBuf, String)> {
        let mut out: Vec<(PathBuf, String)> = Vec::new();
        for (i, step) in self.steps.iter().enumerate() {
            let w = wanted.get(i).copied().unwrap_or(true);
            if !w || step.no_op {
                continue;
            }
            match step.step.verb {
                Verb::Create | Verb::Replace | Verb::Overwrite | Verb::Append => {
                    if let Some(slot) = out.iter_mut().find(|(p, _)| *p == step.path) {
                        slot.1 = step.after.clone();
                    } else {
                        out.push((step.path.clone(), step.after.clone()));
                    }
                }
                _ => {}
            }
        }
        out
    }
}

pub fn play_forward(steps: Vec<Step>, cwd: &Path) -> Result<Chain, String> {
    let mut files: HashMap<PathBuf, Option<String>> = HashMap::new();
    let mut snapshots: HashMap<PathBuf, Option<String>> = HashMap::new();
    let mut order: Vec<PathBuf> = Vec::new();
    let mut dirs: HashSet<PathBuf> = HashSet::new();
    let mut applied: Vec<AppliedStep> = Vec::new();

    let total = steps.len();
    let mut failures: Vec<String> = Vec::new();
    for (idx, step) in steps.into_iter().enumerate() {
        let step_no = idx + 1;
        let verb = verb_name(step.verb);
        let step_path = step.path.clone();

        let result = play_one(
            step,
            cwd,
            &mut files,
            &mut snapshots,
            &mut order,
            &mut dirs,
            &mut applied,
        );
        if let Err(reason) = result {
            failures.push(format!(
                "step {} of {} ({} {}) failed: {}",
                step_no, total, verb, step_path, reason
            ));
        }
    }
    if !failures.is_empty() {
        return Err(failures.join("; "));
    }
    let clashes = exclusivity_failures(&applied);
    if !clashes.is_empty() {
        return Err(clashes.join("; "));
    }

    // Move detection across all steps at once. Each step's rows plus its
    // display path go in; the returned per-step spans get stored on the
    // AppliedSteps.
    let step_inputs: Vec<(Vec<crate::text_diff::DiffRow>, String)> = applied
        .iter()
        .map(|s| {
            // An astgrep step keeps its slot so indices stay aligned, but
            // gives move detection nothing to look at.
            let rows = if s.step.verb == Verb::AstGrep {
                Vec::new()
            } else {
                let before = s.before.as_deref().unwrap_or("");
                pair_lines_full(before, &s.after)
            };
            (rows, s.path.display().to_string())
        })
        .collect();
    let all_spans = move_detect::detect(&step_inputs);
    for (i, spans) in all_spans.into_iter().enumerate() {
        if let Some(slot) = applied.get_mut(i) {
            if !spans.is_empty() {
                let before = slot.before.as_deref().unwrap_or("");
                let flat = pair_lines(before, &slot.after);
                let new_rows = crate::text_diff::move_display::apply(flat, &spans);
                slot.rows = process(&new_rows);
            }
            slot.move_spans = spans;
        }
    }

    let snapshots = order
        .into_iter()
        .map(|p| Snapshot {
            content: snapshots.get(&p).cloned().flatten(),
            path: p,
        })
        .collect();

    let boundary = common_folder(applied.iter().flat_map(|s| {
        let mut v: Vec<&Path> = if s.files.is_empty() {
            vec![s.path.as_path()]
        } else {
            s.files.iter().map(|f| f.path.as_path()).collect()
        };
        if let Some(d) = &s.dest {
            v.push(d.as_path());
        }
        v
    }));

    Ok(Chain {
        steps: applied,
        snapshots,
        notify: None,
        boundary,
        review_indices: Vec::new(),
        overrides: Vec::new(),
    })
}

/// The text the pen shows for a step: the bare new name for a rename,
/// the full target path otherwise.
pub fn target_draft(applied: &AppliedStep) -> String {
    match applied.step.verb {
        Verb::Rename => applied
            .dest
            .as_ref()
            .and_then(|d| d.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        _ => applied.path.display().to_string(),
    }
}

/// Re-plan the chain with one step's target replaced. Clones every
/// applied step, sets the edited one's target with Step::set_target,
/// remaps every later step that pointed at the old target, runs
/// play_forward, and returns the new chain with the old chain's notify
/// and overrides carried over. The caller validates the input is a legal
/// target for the verb; this function does that too and returns the
/// sentence otherwise.
pub fn replan_with_target(old: &Chain, step_index: usize, text: &str) -> Result<Chain, String> {
    let Some(edited) = old.steps.get(step_index) else {
        return Err("no such step".to_string());
    };
    if !edited.step.verb.has_editable_target() {
        return Err(format!(
            "{} steps have no editable target",
            edited.step.verb.name()
        ));
    }
    let text = text.trim();
    if text.is_empty() {
        return Err("the target is empty".to_string());
    }
    let cwd = Path::new("/");
    let mut new_step = edited.step.clone();
    let target_form: String = match new_step.verb {
        Verb::Rename => {
            if text.contains('/') {
                return Err("the new name must be a bare file name".to_string());
            }
            if text == "." || text == ".." {
                return Err("the new name must be a bare file name".to_string());
            }
            let parent = Path::new(&new_step.path)
                .parent()
                .ok_or_else(|| "rename: no parent folder".to_string())?;
            let dest = parent.join(text);
            if let Some(msg) = zap_wire::hidden::HiddenSet::check(&dest) {
                return Err(msg);
            }
            let dest_str = dest.to_string_lossy().into_owned();
            new_step.set_target(text)?;
            dest_str
        }
        Verb::Create | Verb::Mkdir | Verb::Move => {
            if !(text == "~" || text.starts_with("~/") || text.starts_with('/')) {
                return Err(format!(
                    "{}: path must be absolute or start with ~/ (got {})",
                    new_step.verb.name(),
                    text
                ));
            }
            let path = Path::new(text);
            if let Some(msg) = zap_wire::hidden::HiddenSet::check(path) {
                return Err(msg);
            }
            if let Some(msg) =
                zap_wire::extensions::ExtensionAllowlist::load().check(new_step.verb.name(), path)
            {
                return Err(msg);
            }
            new_step.set_target(text)?;
            text.to_string()
        }
        _ => {
            return Err(format!(
                "{} steps have no editable target",
                new_step.verb.name()
            ));
        }
    };

    let old_target: PathBuf = match edited.step.verb {
        Verb::Rename | Verb::Move => edited.dest.clone().unwrap_or_else(|| edited.path.clone()),
        Verb::Create | Verb::Mkdir => edited.path.clone(),
        _ => return Err("no editable target".to_string()),
    };
    let new_target: PathBuf = match new_step.verb {
        Verb::Rename | Verb::Move => {
            let d = new_step
                .dest
                .as_ref()
                .ok_or_else(|| "missing new dest".to_string())?;
            resolve(d, cwd)
        }
        Verb::Create | Verb::Mkdir => resolve(&new_step.path, cwd),
        _ => return Err("no editable target".to_string()),
    };
    let is_dir_like = matches!(new_step.verb, Verb::Mkdir);

    let mut steps: Vec<Step> = Vec::with_capacity(old.steps.len());
    let mut followers = 0usize;
    for (i, applied) in old.steps.iter().enumerate() {
        let mut s = applied.step.clone();
        if i == step_index {
            s = new_step.clone();
        } else if i > step_index {
            let old_p = resolve(&s.path, cwd);
            if old_p == old_target || (is_dir_like && old_p.starts_with(&old_target)) {
                let suffix = old_p
                    .strip_prefix(&old_target)
                    .unwrap_or_else(|_| Path::new(""));
                let rewritten = if suffix.as_os_str().is_empty() {
                    new_target.clone()
                } else {
                    new_target.join(suffix)
                };
                s.path = rewritten.to_string_lossy().into_owned();
                followers += 1;
            }
            if let Some(d) = s.dest.clone() {
                let dp = resolve(&d, cwd);
                if dp == old_target || (is_dir_like && dp.starts_with(&old_target)) {
                    let suffix = dp
                        .strip_prefix(&old_target)
                        .unwrap_or_else(|_| Path::new(""));
                    let rewritten = if suffix.as_os_str().is_empty() {
                        new_target.clone()
                    } else {
                        new_target.join(suffix)
                    };
                    s.dest = Some(rewritten.to_string_lossy().into_owned());
                    followers += 1;
                }
            }
            if s.verb.is_run() {
                if let Some(first) = s.args.first().cloned() {
                    let ap = resolve(&first, cwd);
                    if ap == old_target {
                        s.args[0] = new_target.to_string_lossy().into_owned();
                        followers += 1;
                    }
                }
            }
        }
        steps.push(s);
    }

    let mut chain = play_forward(steps, cwd)?;
    chain.review_indices = old.review_indices.clone();
    if let Some(m) = &old.notify {
        if let Ok(g) = m.lock() {
            chain.notify = Some(Mutex::new(g.clone()));
        }
    }
    chain.overrides = old.overrides.clone();
    if let Some(existing) = chain.overrides.iter_mut().find(|o| o.step == step_index) {
        existing.to = target_form;
        existing.followers += followers;
    } else {
        chain.overrides.push(Override {
            step: step_index,
            from: old_target.to_string_lossy().into_owned(),
            to: target_form,
            followers,
        });
    }
    Ok(chain)
}

/// How often `new` already appears as a whole word in the texts the
/// rename starts from.
fn collision_count(edits: &[astgrep::FileEdit], new: &str) -> usize {
    edits
        .iter()
        .map(|e| tokenize(&e.before).iter().filter(|t| **t == new).count())
        .sum()
}

/// The changed files of a rename end to end, each preceded by a line
/// naming it, so the full-file toggle and anything that reads
/// before/after keeps working.
fn join_files(edits: &[astgrep::FileEdit], after: bool) -> String {
    let mut out = String::new();
    for e in edits {
        out.push_str(&format!("=== {} ===\n", e.path.display()));
        let text = if after { &e.after } else { &e.before };
        out.push_str(text);
        if !text.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

/// Rows for a rename: a summary, then per file a header row (path
/// relative to the folder, occurrence count) and that file's hunks.
fn astgrep_rows(
    folder: &Path,
    old: &str,
    new: &str,
    edits: &[astgrep::FileEdit],
) -> Vec<DiffRowProcessedForUi> {
    let occurrences: usize = edits.iter().map(|e| e.occurrences).sum();
    let collisions = collision_count(edits, new);
    let mut summary = format!(
        "rename identifier {old} -> {new}: {occurrences} occurrences in {} files under {}",
        edits.len(),
        folder.display()
    );
    if collisions > 0 {
        summary.push_str(&format!(
            "\nnote: {new} already appears as a whole word {collisions} time(s) in these files"
        ));
    }
    let mut rows = process(&pair_lines_full(&summary, &summary));
    for e in edits {
        let rel = e
            .path
            .strip_prefix(folder)
            .unwrap_or(&e.path)
            .display()
            .to_string();
        let header = format!("{rel}: {} occurrence(s)", e.occurrences);
        rows.extend(process(&pair_lines_full(&header, &header)));
        rows.extend(process(&astgrep_pair_by_index(&e.before, &e.after)));
    }
    rows
}

/// Pair lines by index. Astgrep only replaces one token per occurrence,
/// so before and after have the same line count and each changed line
/// differs from its counterpart by the renamed token alone. The normal
/// aligner would treat the whole run as one gap and, since "old_name"
/// vs "new_name" scores below SIMILARITY_THRESHOLD, emit Delete-then-
/// Insert for every line. Pairing by index makes each changed line a
/// Change row, which is what a token rename is.
///
/// The result is a snippet: changed lines plus three lines of context
/// on each side, like pair_lines. Falls back to pair_lines when the
/// line counts differ.
fn astgrep_pair_by_index(before: &str, after: &str) -> Vec<DiffRow> {
    let before_lines: Vec<&str> = before.lines().collect();
    let after_lines: Vec<&str> = after.lines().collect();
    if before_lines.len() != after_lines.len() {
        return pair_lines(before, after);
    }
    let mut keep = vec![false; before_lines.len()];
    let mut any_change = false;
    for i in 0..before_lines.len() {
        if before_lines[i] != after_lines[i] {
            any_change = true;
            let lo = i.saturating_sub(3);
            let hi = (i + 4).min(before_lines.len());
            for k in lo..hi {
                keep[k] = true;
            }
        }
    }
    if !any_change {
        return pair_lines(before, after);
    }
    let mut out: Vec<DiffRow> = Vec::new();
    for i in 0..before_lines.len() {
        if !keep[i] {
            continue;
        }
        let b = before_lines[i];
        let a = after_lines[i];
        let lineno = (i + 1) as u32;
        let mut left = Side::new(Some(lineno), b.to_string());
        let mut right = Side::new(Some(lineno), a.to_string());
        let kind = if b == a {
            RowKind::Equal
        } else {
            RowKind::Change
        };
        let mut edit = false;
        if kind == RowKind::Change {
            let (lch, rch) = inline_spans(b, a);
            left.changed = lch;
            right.changed = rch;
            edit = line_similarity(b, a) >= EDIT_THRESHOLD;
        }
        let mut row = DiffRow::new(Some(left), Some(right), kind);
        row.edit = edit;
        out.push(row);
    }
    out
}

/// astgrep rewrites files it found by scanning the disk, so no other step
/// may touch them. A step before it may not touch any .rs under its
/// folder, and a step after it may not touch a file it changed.
fn exclusivity_failures(applied: &[AppliedStep]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let Some(ai) = applied
        .iter()
        .position(|s| s.step.verb == Verb::AstGrep)
    else {
        return out;
    };
    let folder = applied[ai].path.clone();
    let changed: Vec<&Path> = applied[ai].files.iter().map(|f| f.path.as_path()).collect();
    for (k, s) in applied.iter().enumerate() {
        if k == ai {
            continue;
        }
        let touched: Vec<&Path> = std::iter::once(s.path.as_path())
            .chain(s.dest.as_deref())
            .collect();
        let clash = if k < ai {
            touched.iter().find(|p| {
                p.starts_with(&folder) && p.extension().and_then(|e| e.to_str()) == Some("rs")
            })
        } else {
            touched.iter().find(|p| changed.contains(*p))
        };
        if let Some(p) = clash {
            out.push(format!(
                "step {} ({} {}) touches {}, which astgrep rewrites, put it in a separate paste",
                k + 1,
                verb_name(s.step.verb),
                s.path.display(),
                p.display()
            ));
        }
    }
    out
}

/// Deepest common folder of every path's parent, by whole components.
/// Returns `/` when the paths don't share a parent (or the list is empty).
fn common_folder<'a, I: IntoIterator<Item = &'a Path>>(paths: I) -> PathBuf {
    let mut iter = paths.into_iter();
    let Some(first) = iter.next() else {
        return PathBuf::from("/");
    };
    let Some(first_parent) = first.parent() else {
        return PathBuf::from("/");
    };
    let mut common: Vec<std::path::Component> = first_parent.components().collect();
    for p in iter {
        let Some(parent) = p.parent() else {
            return PathBuf::from("/");
        };
        let comps: Vec<std::path::Component> = parent.components().collect();
        let mut i = 0;
        while i < common.len() && i < comps.len() && common[i] == comps[i] {
            i += 1;
        }
        common.truncate(i);
        if common.is_empty() {
            return PathBuf::from("/");
        }
    }
    if common.is_empty() {
        return PathBuf::from("/");
    }
    let mut out = PathBuf::new();
    for c in common {
        out.push(c.as_os_str());
    }
    if out.as_os_str().is_empty() {
        PathBuf::from("/")
    } else {
        out
    }
}

/// Steps that need eyes in the review window. Three reasons:
/// a verb reason (always_reviewed), a whitelist reason (the step's path,
/// or a move's destination, is not matched by the whitelist), or an
/// approval reason (the path's final write is rejected by `is_unapproved`,
/// a closure over (path, final text) that knows about .sh / .toml and the
/// stored approvals). A verb reason marks that step only. A path reason
/// marks every step that touches that path, because the reviewer cannot
/// judge the last edit to a file without the earlier edits of it in the
/// same chain.
///
/// Empty means nothing needs eyes and the caller should auto-apply as a
/// fully whitelisted chain does. The full step list means every step
/// needs eyes and the caller should stash with `review_indices` empty.
/// Anything between means only those steps are shown.
pub fn steps_that_need_eyes<F>(chain: &Chain, whitelist: &[PathBuf], is_unapproved: F) -> Vec<usize>
where
    F: Fn(&Path, &str) -> bool,
{
    let mut marked: HashSet<usize> = HashSet::new();
    for (i, s) in chain.steps.iter().enumerate() {
        if s.step.verb.always_reviewed() {
            marked.insert(i);
        }
    }

    let writes = chain.final_writes(&[]);
    let final_write_for =
        |p: &Path| -> Option<&String> { writes.iter().find(|(wp, _)| wp == p).map(|(_, t)| t) };

    let mut path_needs_eyes: HashSet<PathBuf> = HashSet::new();
    for s in chain.steps.iter() {
        for p in std::iter::once(&s.path).chain(s.dest.as_ref()) {
            let whitelist_ok =
                !whitelist.is_empty() && crate::chain::whitelist::is_whitelisted(p, whitelist);
            if !whitelist_ok {
                path_needs_eyes.insert(p.clone());
                continue;
            }
            if let Some(text) = final_write_for(p) {
                if is_unapproved(p, text) {
                    path_needs_eyes.insert(p.clone());
                }
            }
        }
    }

    for (i, s) in chain.steps.iter().enumerate() {
        for p in std::iter::once(&s.path).chain(s.dest.as_ref()) {
            if path_needs_eyes.contains(p) {
                marked.insert(i);
            }
        }
    }

    let mut out: Vec<usize> = marked.into_iter().collect();
    out.sort_unstable();
    out
}

fn verb_name(v: Verb) -> &'static str {
    match v {
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

#[allow(clippy::too_many_arguments)]
fn play_one(
    step: Step,
    cwd: &Path,
    files: &mut HashMap<PathBuf, Option<String>>,
    snapshots: &mut HashMap<PathBuf, Option<String>>,
    order: &mut Vec<PathBuf>,
    dirs: &mut HashSet<PathBuf>,
    applied: &mut Vec<AppliedStep>,
) -> Result<(), String> {
    let path = resolve(&step.path, cwd);

    match step.verb {
        Verb::Mkdir => {
            if path.exists()
                || files.get(&path).cloned().flatten().is_some()
                || dirs.contains(&path)
            {
                return Err(format!("{} already exists", path.display()));
            }
            dirs.insert(path.clone());
            let after = format!("(new folder) {}/", path.display());
            let rows = process(&pair_lines("", &after));
            applied.push(AppliedStep {
                step,
                path,
                dest: None,
                before: None,
                after,
                rows,
                move_spans: Vec::new(),
                files: Vec::new(),
                no_op: false,
            });
        }
        Verb::RemoveDir => {
            // If an earlier step in this chain made the folder, its real
            // path may not exist yet. Trust the in-memory dir set then.
            if dirs.contains(&path) {
                dirs.remove(&path);
            } else {
                if !path.is_dir() {
                    return Err(format!(
                        "{} does not exist or is not a folder",
                        path.display()
                    ));
                }
                let mut entries = std::fs::read_dir(&path)
                    .map_err(|e| format!("read_dir {}: {}", path.display(), e))?;
                if entries.next().is_some() {
                    return Err(format!("{} is not empty", path.display()));
                }
            }
            let after = format!("(removed folder) {}/", path.display());
            let rows = process(&pair_lines(&after, ""));
            applied.push(AppliedStep {
                step,
                path,
                dest: None,
                before: Some(after),
                after: String::new(),
                rows,
                move_spans: Vec::new(),
                files: Vec::new(),
                no_op: false,
            });
        }
        Verb::Remove => {
            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.is_dir() {
                    return Err(format!("{}: is a directory", path.display()));
                }
                if meta.len() > RM_MAX_BYTES {
                    return Err(
                        "can't keep an undo copy of this file; delete it by hand".to_string()
                    );
                }
            }
            load_once(&path, files, snapshots, order)?;
            let before = files.get(&path).cloned().flatten();
            let Some(content) = before.clone() else {
                return Err(with_hint(
                    format!("{} does not exist", path.display()),
                    missing_file_hint(&path, files, dirs),
                ));
            };
            let rows = process(&pair_lines(&content, ""));
            files.insert(path.clone(), None);
            applied.push(AppliedStep {
                step,
                path,
                dest: None,
                before: Some(content),
                after: String::new(),
                rows,
                move_spans: Vec::new(),
                files: Vec::new(),
                no_op: false,
            });
        }
        Verb::Move | Verb::Rename => {
            load_once(&path, files, snapshots, order)?;
            let dest_str = step
                .dest
                .as_ref()
                .ok_or_else(|| "missing destination".to_string())?;
            let dest = resolve(dest_str, cwd);
            if dest == path {
                return Err(format!("{} is the same as the source", path.display()));
            }
            let source_content = files.get(&path).cloned().flatten();
            let Some(content) = source_content.clone() else {
                return Err(with_hint(
                    format!("{} does not exist", path.display()),
                    missing_file_hint(&path, files, dirs),
                ));
            };
            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.is_dir() {
                    return Err(format!("{}: is a directory", path.display()));
                }
            }
            if files.get(&dest).cloned().flatten().is_some() || dest.exists() {
                return Err(format!("{} already exists", dest.display()));
            }
            if !files.contains_key(&dest) {
                files.insert(dest.clone(), None);
                snapshots.insert(dest.clone(), None);
                order.push(dest.clone());
            }
            files.insert(dest.clone(), Some(content.clone()));
            let line_count = content.lines().count();
            let rows = process(&path_change_rows(&path, &dest, line_count));
            files.insert(path.clone(), None);
            applied.push(AppliedStep {
                step,
                path,
                dest: Some(dest.clone()),
                before: Some(content.clone()),
                after: content,
                rows,
                move_spans: Vec::new(),
                files: Vec::new(),
                no_op: false,
            });
        }
        Verb::Create | Verb::Replace | Verb::Overwrite | Verb::Append => {
            load_once(&path, files, snapshots, order)?;
            let current = files.get(&path).cloned().flatten();
            let (after, no_op) = match apply_step(&step, current.as_deref(), &path) {
                Ok(v) => v,
                Err(reason) => {
                    let hint = failure_hint(&step, &path, current.as_deref(), files, dirs);
                    return Err(with_hint(reason, hint));
                }
            };
            let before = current;
            let rows = match &before {
                Some(b) => process(&pair_lines(b, &after)),
                None => process(&pair_lines("", &after)),
            };
            files.insert(path.clone(), Some(after.clone()));
            applied.push(AppliedStep {
                step,
                path,
                dest: None,
                before,
                after,
                rows,
                move_spans: Vec::new(),
                files: Vec::new(),
                no_op,
            });
        }
        Verb::Chmod => {
            load_once(&path, files, snapshots, order)?;
            let content = files.get(&path).cloned().flatten();
            let Some(content) = content else {
                return Err(with_hint(
                    format!("{} does not exist", path.display()),
                    missing_file_hint(&path, files, dirs),
                ));
            };
            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.is_dir() {
                    return Err(format!("{}: is a directory", path.display()));
                }
            }
            let rows = process(&pair_lines_full(&content, &content));
            applied.push(AppliedStep {
                step,
                path,
                dest: None,
                before: Some(content.clone()),
                after: content,
                rows,
                move_spans: Vec::new(),
                files: Vec::new(),
                no_op: false,
            });
        }
        Verb::Cargo | Verb::Run => {
            let args = step.args.join(" ");
            let folder_raw = step.cwd.clone().unwrap_or_else(|| step.path.clone());
            let folder = resolve(&folder_raw, cwd);
            let cmd_line = match step.verb {
                Verb::Cargo => format!("cargo {args}"),
                Verb::Run => format!("run {args}"),
                _ => unreachable!(),
            };
            let after = format!("{cmd_line}\nin {}", folder.display());
            let rows = process(&pair_lines("", &after));
            applied.push(AppliedStep {
                step,
                path: folder,
                dest: None,
                before: None,
                after,
                rows,
                move_spans: Vec::new(),
                files: Vec::new(),
                no_op: false,
            });
        }
        Verb::AstGrep => {
            if !path.is_dir() {
                return Err(format!(
                    "{} does not exist or is not a folder",
                    path.display()
                ));
            }
            let old_name = step.old_text.clone();
            let new_name = step.new_text.clone();
            let hidden = zap_wire::hidden::HiddenSet::load();
            let edits = {
                let mut read = |p: &Path| -> Result<Option<String>, String> {
                    load_once(p, files, snapshots, order)?;
                    Ok(files.get(p).cloned().flatten())
                };
                astgrep::plan(&path, &old_name, &new_name, &hidden, &mut read)?
            };
            for e in &edits {
                files.insert(e.path.clone(), Some(e.after.clone()));
            }
            let before = join_files(&edits, false);
            let after = join_files(&edits, true);
            let rows = astgrep_rows(&path, &old_name, &new_name, &edits);
            applied.push(AppliedStep {
                step,
                path,
                dest: None,
                before: Some(before),
                after,
                rows,
                move_spans: Vec::new(),
                files: edits,
                no_op: false,
            });
        }
    }

    Ok(())
}

// ---- hints on failed steps ----
//
// A hint is one parenthesised clause appended to a failure reason. It
// never holds a newline or a semicolon of its own. apply_step stays pure
// and keeps its messages; play_one adds the hint around its Err.

/// No hint on a replace in a file with more lines than this.
const HINT_MAX_FILE_LINES: usize = 20_000;
/// Directory entries scanned when looking for a similar file name.
const HINT_MAX_NAMES: usize = 5_000;
/// Most match positions listed for a wrong-count replace.
const HINT_MAX_POSITIONS: usize = 5;

fn with_hint(reason: String, hint: Option<String>) -> String {
    match hint {
        Some(h) => format!("{reason} ({h})"),
        None => reason,
    }
}

/// File text quoted in a hint: trimmed, first line only, cut to 80
/// characters, in backticks.
fn quote_line(s: &str) -> String {
    format!("`{}`", truncate(s.trim(), 80))
}

fn line_of(content: &str, byte: usize) -> usize {
    content[..byte].bytes().filter(|b| *b == b'\n').count() + 1
}

/// The hint for a failed content step, from the text the step saw.
fn failure_hint(
    step: &Step,
    path: &Path,
    current: Option<&str>,
    files: &HashMap<PathBuf, Option<String>>,
    dirs: &HashSet<PathBuf>,
) -> Option<String> {
    match (step.verb, current) {
        (Verb::Create, Some(c)) => Some(create_hint(c)),
        (Verb::Overwrite | Verb::Append | Verb::Replace, None) => {
            missing_file_hint(path, files, dirs)
        }
        (Verb::Replace, Some(c)) => replace_hint(c, &step.old_text, step.count.unwrap_or(1)),
        _ => None,
    }
}

fn create_hint(current: &str) -> String {
    let n = current.lines().count();
    if n == 0 {
        return "it is empty, use overwrite to replace it".to_string();
    }
    let first = current.lines().next().unwrap_or("");
    let unit = if n == 1 { "line" } else { "lines" };
    format!(
        "it has {n} {unit} and starts with {}, use overwrite to replace it",
        quote_line(first)
    )
}

/// Hint for a replace whose old text did not occur `expected` times.
fn replace_hint(content: &str, old: &str, expected: usize) -> Option<String> {
    if old.is_empty() || content.lines().count() > HINT_MAX_FILE_LINES {
        return None;
    }
    let found = content.matches(old).count();
    if found == expected {
        return None;
    }
    if found > 0 {
        let at: Vec<usize> = content
            .match_indices(old)
            .map(|(i, _)| line_of(content, i))
            .take(HINT_MAX_POSITIONS)
            .collect();
        let list = at
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Some(if found > at.len() {
            format!("first {} at lines {list}", at.len())
        } else if at.len() == 1 {
            format!("at line {list}")
        } else {
            format!("at lines {list}")
        });
    }

    let file_lines: Vec<&str> = content.lines().collect();
    let old_lines: Vec<&str> = old.lines().collect();
    if old_lines.is_empty() || old_lines.iter().all(|l| l.trim().is_empty()) {
        return None;
    }

    // Tier 1: the same lines once indentation, tabs and line endings go.
    if let Some(n) = whitespace_match(&file_lines, &old_lines) {
        return Some(format!(
            "the old text matches at line {n} when indentation, tabs or line endings are ignored"
        ));
    }

    // Tier 2: the closest line.
    if old_lines.iter().all(|l| tokenize(l).is_empty()) {
        return None;
    }
    match closest_line(&old_lines, &file_lines) {
        Some(c) if c.score >= SIMILARITY_THRESHOLD => {
            Some(describe_closest(&file_lines, &old_lines, c))
        }
        _ => Some(
            "no line in the file is close to the old text, so the path may be wrong".to_string(),
        ),
    }
}

/// 1-based line where a run of file lines equals the old lines, trimmed
/// at both ends. `str::lines` already drops a carriage return.
fn whitespace_match(file: &[&str], old: &[&str]) -> Option<usize> {
    let m = old.len();
    if m == 0 || file.len() < m {
        return None;
    }
    let first = old[0].trim();
    for s in 0..=(file.len() - m) {
        if file[s].trim() != first {
            continue;
        }
        if (1..m).all(|j| file[s + j].trim() == old[j].trim()) {
            return Some(s + 1);
        }
    }
    None
}

fn describe_closest(file: &[&str], old: &[&str], c: ClosestLine) -> String {
    let anchor_msg = format!(
        "old-text line {} looks closest to line {}: {}",
        c.old_index + 1,
        c.file_index + 1,
        quote_line(file[c.file_index])
    );
    if old.len() == 1 {
        return format!(
            "line {} looks closest: {}",
            c.file_index + 1,
            quote_line(file[c.file_index])
        );
    }
    // Shift the old text so its anchor sits on the hit, then walk it.
    let start = c.file_index as isize - c.old_index as isize;
    for (j, ol) in old.iter().enumerate() {
        let fi = start + j as isize;
        if fi < 0 {
            return anchor_msg;
        }
        let fi = fi as usize;
        if fi >= file.len() {
            return format!("the file ends before old-text line {}", j + 1);
        }
        if file[fi].trim() != ol.trim() {
            return format!(
                "old-text line {} differs from line {} of the file, which reads {}",
                j + 1,
                fi + 1,
                quote_line(file[fi])
            );
        }
    }
    anchor_msg
}

/// Closest name by edit distance among `names`. Equal ignoring case wins;
/// otherwise a distance of at most 2 or a fifth of the name's length;
/// ties go to the alphabetically first.
fn suggest_name<'a>(name: &str, names: &'a [String]) -> Option<&'a str> {
    let mut sorted: Vec<&String> = names.iter().filter(|n| n.as_str() != name).collect();
    sorted.sort();
    let lower = name.to_lowercase();
    if let Some(n) = sorted.iter().find(|n| n.to_lowercase() == lower) {
        return Some(n.as_str());
    }
    let limit = 2usize.max(name.chars().count() / 5);
    let mut best: Option<(&String, usize)> = None;
    for n in sorted {
        let d = edit_distance(name, n);
        if d <= limit && best.map_or(true, |(_, bd)| d < bd) {
            best = Some((n, d));
        }
    }
    best.map(|(n, _)| n.as_str())
}

/// Hint for a path that does not exist: a similar name in the same
/// folder (on disk or made by an earlier step), or, when the folder
/// itself is missing, the nearest folder that exists.
fn missing_file_hint(
    path: &Path,
    files: &HashMap<PathBuf, Option<String>>,
    dirs: &HashSet<PathBuf>,
) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let parent = path.parent()?;

    let mut names: Vec<String> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(parent) {
        for e in rd.flatten().take(HINT_MAX_NAMES) {
            let n = e.file_name().to_string_lossy().into_owned();
            // An earlier step in this chain removed or moved it away.
            if matches!(files.get(&parent.join(&n)), Some(None)) {
                continue;
            }
            names.push(n);
        }
    }
    let mut made_here = false;
    for (p, content) in files {
        if content.is_some() && p.parent() == Some(parent) {
            if let Some(n) = p.file_name().and_then(|n| n.to_str()) {
                names.push(n.to_string());
                made_here = true;
            }
        }
    }
    for d in dirs {
        if d.parent() == Some(parent) {
            if let Some(n) = d.file_name().and_then(|n| n.to_str()) {
                names.push(n.to_string());
                made_here = true;
            }
        }
    }

    if !parent.is_dir() && !made_here && !dirs.contains(parent) {
        let mut anc = parent.parent();
        while let Some(a) = anc {
            if a.is_dir() {
                return Some(format!(
                    "the folder {} does not exist, the nearest existing folder is {}",
                    parent.display(),
                    a.display()
                ));
            }
            anc = a.parent();
        }
        return Some(format!("the folder {} does not exist", parent.display()));
    }

    names.sort();
    names.dedup();
    suggest_name(name, &names).map(|n| format!("did you mean `{n}`?"))
}

fn load_once(
    path: &Path,
    files: &mut HashMap<PathBuf, Option<String>>,
    snapshots: &mut HashMap<PathBuf, Option<String>>,
    order: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if files.contains_key(path) {
        return Ok(());
    }
    let content = read_or_none(path)?;
    files.insert(path.to_path_buf(), content.clone());
    snapshots.insert(path.to_path_buf(), content);
    order.push(path.to_path_buf());
    Ok(())
}

pub fn apply_step(
    step: &Step,
    current: Option<&str>,
    path: &Path,
) -> Result<(String, bool), String> {
    match step.verb {
        Verb::Create => {
            if current.is_some() {
                return Err(format!("{} already exists", path.display()));
            }
            Ok((step.new_text.clone(), false))
        }
        Verb::Overwrite => {
            let content = current.ok_or_else(|| format!("{} does not exist", path.display()))?;
            if content == step.new_text {
                return Ok((content.to_string(), true));
            }
            Ok((step.new_text.clone(), false))
        }
        Verb::Replace => {
            let content = current.ok_or_else(|| format!("{} does not exist", path.display()))?;
            if step.old_text.is_empty() {
                return Err("old text is empty".to_string());
            }
            let expected = step.count.unwrap_or(1);
            let found = content.matches(step.old_text.as_str()).count();
            if found != expected {
                return Err(format!(
                    "expected {} occurrence(s) of the old text in {}, found {}",
                    expected,
                    path.display(),
                    found
                ));
            }
            let result = content.replace(step.old_text.as_str(), step.new_text.as_str());
            if result == content {
                return Ok((content.to_string(), true));
            }
            Ok((result, false))
        }
        Verb::Append => {
            let content = current.ok_or_else(|| format!("{} does not exist", path.display()))?;
            if step.new_text.is_empty() {
                return Ok((content.to_string(), true));
            }
            let mut out = content.to_string();
            if step.no_newline {
                out.push_str(&step.new_text);
            } else {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                out.push_str(&step.new_text);
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            if out == content {
                return Ok((content.to_string(), true));
            }
            Ok((out, false))
        }
        Verb::Remove
        | Verb::Move
        | Verb::Rename
        | Verb::Mkdir
        | Verb::RemoveDir
        | Verb::Chmod
        | Verb::Cargo
        | Verb::Run
        | Verb::AstGrep => Err("apply_step: this verb is handled in play_forward".to_string()),
    }
}

/// Two small texts that show a path change as a diff: old path then
/// "(contents unchanged, N lines)" on the left, new path then the same
/// note on the right. Used by Move and Rename so both verbs look alike
/// in the review window without inventing content rows.
pub fn path_change_rows(
    from: &Path,
    to: &Path,
    line_count: usize,
) -> Vec<crate::text_diff::DiffRow> {
    let note = format!("(contents unchanged, {line_count} lines)");
    let before = format!("{}\n{note}", from.display());
    let after = format!("{}\n{note}", to.display());
    pair_lines(&before, &after)
}

pub fn read_or_none(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("read {}: {}", path.display(), e)),
    }
}

pub fn resolve(path: &str, cwd: &Path) -> PathBuf {
    if path == "~" {
        return home_dir();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    let p = PathBuf::from(path);
    if p.is_absolute() { p } else { cwd.join(p) }
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("zap_play_test_{}_{}", std::process::id(), nanos));
        fs::create_dir_all(&p).expect("mkdir");
        p
    }

    fn text_step(verb: Verb, path: &Path, old: &str, new: &str) -> Step {
        Step {
            verb,
            path: path.display().to_string(),
            dest: None,
            old_text: old.to_string(),
            new_text: new.to_string(),
            count: None,
            no_newline: false,
            args: Vec::new(),
            cwd: None,
        }
    }

    fn mv_step(from: &Path, to: &Path) -> Step {
        Step {
            verb: Verb::Move,
            path: from.display().to_string(),
            dest: Some(to.display().to_string()),
            old_text: String::new(),
            new_text: String::new(),
            count: None,
            no_newline: false,
            args: Vec::new(),
            cwd: None,
        }
    }

    fn rm_step(path: &Path) -> Step {
        Step {
            verb: Verb::Remove,
            path: path.display().to_string(),
            dest: None,
            old_text: String::new(),
            new_text: String::new(),
            count: None,
            no_newline: false,
            args: Vec::new(),
            cwd: None,
        }
    }

    fn mkdir_step(path: &Path) -> Step {
        Step {
            verb: Verb::Mkdir,
            path: path.display().to_string(),
            dest: None,
            old_text: String::new(),
            new_text: String::new(),
            count: None,
            no_newline: false,
            args: Vec::new(),
            cwd: None,
        }
    }

    fn rmdir_step(path: &Path) -> Step {
        Step {
            verb: Verb::RemoveDir,
            path: path.display().to_string(),
            dest: None,
            old_text: String::new(),
            new_text: String::new(),
            count: None,
            no_newline: false,
            args: Vec::new(),
            cwd: None,
        }
    }

    #[test]
    fn create_then_replace_sees_earlier_step() {
        let dir = tmp();
        let file = dir.join("a.txt");
        let steps = vec![
            text_step(Verb::Create, &file, "", "hello\n"),
            text_step(Verb::Replace, &file, "hello", "world"),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        assert_eq!(chain.steps.len(), 2);
        assert_eq!(chain.steps[1].after, "world\n");
        assert_eq!(chain.snapshots.len(), 1);
        assert_eq!(chain.snapshots[0].content, None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_fails_when_old_text_missing() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let steps = vec![text_step(Verb::Replace, &file, "nope", "world")];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("step 1 of 1"));
        assert!(err.contains("replace"));
        assert!(err.contains("expected 1 occurrence"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn failure_names_the_step_index() {
        let dir = tmp();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "hello\n").expect("write");
        let steps = vec![
            text_step(Verb::Replace, &a, "hello", "world"),
            text_step(Verb::Replace, &b, "nope", "world"),
        ];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.starts_with("step 2 of 2"), "got: {err}");
        assert!(err.contains("replace"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reports_every_failure_in_one_sentence() {
        let dir = tmp();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "hello\n").expect("write");
        fs::write(&b, "world\n").expect("write");
        let steps = vec![
            text_step(Verb::Replace, &a, "nope", "x"),
            text_step(Verb::Replace, &b, "missing", "y"),
        ];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("step 1 of 2"), "got: {err}");
        assert!(err.contains("step 2 of 2"), "got: {err}");
        assert!(err.contains("; "), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_honours_count() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "x x x\n").expect("write");
        let mut s = text_step(Verb::Replace, &file, "x", "y");
        s.count = Some(2);
        let steps = vec![s];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("expected 2"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rm_needs_the_file_to_exist() {
        let dir = tmp();
        let file = dir.join("a.txt");
        let steps = vec![rm_step(&file)];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("does not exist"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rm_refuses_directories() {
        let dir = tmp();
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).expect("mkdir");
        let steps = vec![rm_step(&sub)];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("is a directory"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rm_then_create_sees_empty() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let steps = vec![rm_step(&file), text_step(Verb::Create, &file, "", "new\n")];
        let chain = play_forward(steps, &dir).expect("play");
        assert_eq!(chain.steps[1].after, "new\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mv_needs_source_and_dest_checks() {
        let dir = tmp();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "hi\n").expect("write");
        let steps = vec![mv_step(&a, &b)];
        let chain = play_forward(steps, &dir).expect("play");
        assert_eq!(chain.steps.len(), 1);
        assert_eq!(chain.steps[0].after, "hi\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mv_refuses_existing_destination() {
        let dir = tmp();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "hi\n").expect("write");
        fs::write(&b, "other\n").expect("write");
        let steps = vec![mv_step(&a, &b)];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("already exists"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mv_onto_itself_is_an_error() {
        let dir = tmp();
        let a = dir.join("a.txt");
        fs::write(&a, "hi\n").expect("write");
        let steps = vec![mv_step(&a, &a)];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("same as the source"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mkdir_refuses_existing() {
        let dir = tmp();
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).expect("mkdir");
        let steps = vec![mkdir_step(&sub)];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("already exists"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mkdir_then_create_inside() {
        let dir = tmp();
        let sub = dir.join("newdir");
        let file = sub.join("a.txt");
        let steps = vec![mkdir_step(&sub), text_step(Verb::Create, &file, "", "hi\n")];
        let chain = play_forward(steps, &dir).expect("play");
        assert_eq!(chain.steps.len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rmdir_succeeds_on_empty_folder() {
        let dir = tmp();
        let sub = dir.join("olddir");
        fs::create_dir_all(&sub).expect("mkdir");
        let steps = vec![rmdir_step(&sub)];
        let chain = play_forward(steps, &dir).expect("play");
        assert_eq!(chain.steps.len(), 1);
        assert_eq!(chain.steps[0].path, sub);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rmdir_refuses_non_empty() {
        let dir = tmp();
        let sub = dir.join("olddir");
        fs::create_dir_all(&sub).expect("mkdir");
        fs::write(sub.join("a.txt"), "x\n").expect("write");
        let steps = vec![rmdir_step(&sub)];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("not empty"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rmdir_missing_folder_fails() {
        let dir = tmp();
        let sub = dir.join("missing");
        let steps = vec![rmdir_step(&sub)];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("does not exist"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mkdir_then_rmdir_in_one_chain() {
        let dir = tmp();
        let sub = dir.join("newdir");
        let steps = vec![mkdir_step(&sub), rmdir_step(&sub)];
        let chain = play_forward(steps, &dir).expect("play");
        assert_eq!(chain.steps.len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_adds_a_line() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "one\n").expect("write");
        let mut s = text_step(Verb::Append, &file, "", "two");
        s.new_text = "two".to_string();
        let chain = play_forward(vec![s], &dir).expect("play");
        assert_eq!(chain.steps[0].after, "one\ntwo\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_inserts_newline_if_missing() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "one").expect("write");
        let mut s = text_step(Verb::Append, &file, "", "two");
        s.new_text = "two".to_string();
        let chain = play_forward(vec![s], &dir).expect("play");
        assert_eq!(chain.steps[0].after, "one\ntwo\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_no_newline_concatenates() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "one").expect("write");
        let mut s = text_step(Verb::Append, &file, "", "two");
        s.new_text = "two".to_string();
        s.no_newline = true;
        let chain = play_forward(vec![s], &dir).expect("play");
        assert_eq!(chain.steps[0].after, "onetwo");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn overwrite_with_identical_content_is_a_no_op() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let s = text_step(Verb::Overwrite, &file, "", "hello\n");
        let chain = play_forward(vec![s], &dir).expect("play");
        assert!(chain.steps[0].no_op);
        assert_eq!(chain.steps[0].after, "hello\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_with_identical_text_is_a_no_op() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let s = text_step(Verb::Replace, &file, "hello", "hello");
        let chain = play_forward(vec![s], &dir).expect("play");
        assert!(chain.steps[0].no_op);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_empty_text_is_a_no_op() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let s = text_step(Verb::Append, &file, "", "");
        let chain = play_forward(vec![s], &dir).expect("play");
        assert!(chain.steps[0].no_op);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn final_writes_keeps_the_last_step_per_path() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let steps = vec![
            text_step(Verb::Replace, &file, "hello", "world"),
            text_step(Verb::Replace, &file, "world", "again"),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        let writes = chain.final_writes(&[]);
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, file);
        assert_eq!(writes[0].1, "again\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn final_writes_drops_no_op_steps() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let steps = vec![
            text_step(Verb::Replace, &file, "hello", "hello"),
            text_step(Verb::Replace, &file, "hello", "world"),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        let writes = chain.final_writes(&[]);
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].1, "world\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn final_writes_drops_zaprm_zapmv_mkdir_rmdir() {
        let dir = tmp();
        let rm_file = dir.join("rm.txt");
        let src = dir.join("src.txt");
        let dst = dir.join("dst.txt");
        let sub = dir.join("sub");
        fs::write(&rm_file, "x\n").expect("write");
        fs::write(&src, "y\n").expect("write");
        let steps = vec![
            rm_step(&rm_file),
            mv_step(&src, &dst),
            mkdir_step(&sub),
            rmdir_step(&sub),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        let writes = chain.final_writes(&[]);
        assert!(writes.is_empty(), "got: {writes:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn final_writes_falls_back_to_previous_wanted_step() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let steps = vec![
            text_step(Verb::Replace, &file, "hello", "world"),
            text_step(Verb::Replace, &file, "world", "again"),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        let writes = chain.final_writes(&[true, false]);
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].1, "world\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn final_writes_empty_slice_means_every_step() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let steps = vec![
            text_step(Verb::Replace, &file, "hello", "world"),
            text_step(Verb::Replace, &file, "world", "again"),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        let writes = chain.final_writes(&[]);
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].1, "again\n");
        let _ = fs::remove_dir_all(&dir);
    }

    fn chmod_step(path: &Path) -> Step {
        Step {
            verb: Verb::Chmod,
            path: path.display().to_string(),
            dest: None,
            old_text: String::new(),
            new_text: String::new(),
            count: None,
            no_newline: false,
            args: Vec::new(),
            cwd: None,
        }
    }

    #[test]
    fn chmod_on_existing_file_keeps_before_equal_after() {
        let dir = tmp();
        let file = dir.join("a.sh");
        fs::write(&file, "hello\n").expect("write");
        let chain = play_forward(vec![chmod_step(&file)], &dir).expect("play");
        assert_eq!(chain.steps.len(), 1);
        assert_eq!(chain.steps[0].before.as_deref(), Some("hello\n"));
        assert_eq!(chain.steps[0].after, "hello\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn chmod_on_missing_file_fails() {
        let dir = tmp();
        let file = dir.join("a.sh");
        let err = match play_forward(vec![chmod_step(&file)], &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("does not exist"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn chmod_on_directory_fails() {
        let dir = tmp();
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).expect("mkdir");
        let err = match play_forward(vec![chmod_step(&sub)], &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("is a directory"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn chmod_after_create_in_same_chain_succeeds() {
        let dir = tmp();
        let file = dir.join("a.sh");
        let steps = vec![
            text_step(Verb::Create, &file, "", "#!/bin/sh\necho hi\n"),
            chmod_step(&file),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        assert_eq!(chain.steps.len(), 2);
        assert_eq!(chain.steps[1].after, "#!/bin/sh\necho hi\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn chmod_only_chain_needs_review() {
        let dir = tmp();
        let file = dir.join("a.sh");
        fs::write(&file, "x\n").expect("write");
        let chain = play_forward(vec![chmod_step(&file)], &dir).expect("play");
        assert!(chain.needs_review());
        let _ = fs::remove_dir_all(&dir);
    }

    fn no_unapproved(_: &Path, _: &str) -> bool {
        false
    }

    #[test]
    fn three_whitelisted_edits_need_no_eyes() {
        let dir = tmp();
        let sub = dir.join("w");
        fs::create_dir_all(&sub).expect("mkdir");
        let a = sub.join("a.rs");
        let b = sub.join("b.rs");
        let c = sub.join("c.rs");
        fs::write(&a, "a\n").expect("write");
        fs::write(&b, "b\n").expect("write");
        fs::write(&c, "c\n").expect("write");
        let steps = vec![
            text_step(Verb::Replace, &a, "a", "A"),
            text_step(Verb::Replace, &b, "b", "B"),
            text_step(Verb::Replace, &c, "c", "C"),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        let needing = steps_that_need_eyes(&chain, &[sub.clone()], no_unapproved);
        assert!(needing.is_empty(), "got: {needing:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn three_whitelisted_edits_and_a_zaprm_marks_only_the_zaprm() {
        let dir = tmp();
        let sub = dir.join("w");
        fs::create_dir_all(&sub).expect("mkdir");
        let a = sub.join("a.rs");
        let b = sub.join("b.rs");
        let c = sub.join("c.rs");
        let z = sub.join("z.rs");
        for f in [&a, &b, &c, &z] {
            fs::write(f, "x\n").expect("write");
        }
        let steps = vec![
            text_step(Verb::Replace, &a, "x", "A"),
            text_step(Verb::Replace, &b, "x", "B"),
            text_step(Verb::Replace, &c, "x", "C"),
            rm_step(&z),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        let needing = steps_that_need_eyes(&chain, &[sub.clone()], no_unapproved);
        assert_eq!(needing, vec![3]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_edit_outside_whitelist_is_marked_with_the_zaprm() {
        let dir = tmp();
        let sub = dir.join("w");
        let outside = dir.join("out");
        fs::create_dir_all(&sub).expect("mkdir");
        fs::create_dir_all(&outside).expect("mkdir");
        let inside = sub.join("in.rs");
        let out = outside.join("out.rs");
        let z = sub.join("z.rs");
        fs::write(&inside, "x\n").expect("write");
        fs::write(&out, "x\n").expect("write");
        fs::write(&z, "x\n").expect("write");
        let steps = vec![
            text_step(Verb::Replace, &inside, "x", "I"),
            text_step(Verb::Replace, &out, "x", "O"),
            rm_step(&z),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        let needing = steps_that_need_eyes(&chain, &[sub.clone()], no_unapproved);
        assert_eq!(needing, vec![1, 2]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_whitelist_marks_every_step() {
        let dir = tmp();
        let a = dir.join("a.rs");
        let b = dir.join("b.rs");
        fs::write(&a, "x\n").expect("write");
        fs::write(&b, "y\n").expect("write");
        let steps = vec![
            text_step(Verb::Replace, &a, "x", "A"),
            text_step(Verb::Replace, &b, "y", "B"),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        let needing = steps_that_need_eyes(&chain, &[], no_unapproved);
        assert_eq!(needing, vec![0, 1]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unapproved_toml_marks_both_steps_on_it() {
        let dir = tmp();
        let sub = dir.join("w");
        fs::create_dir_all(&sub).expect("mkdir");
        let toml = sub.join("Cargo.toml");
        fs::write(&toml, "a = 1\n").expect("write");
        let steps = vec![
            text_step(Verb::Replace, &toml, "a = 1", "a = 2"),
            text_step(Verb::Replace, &toml, "a = 2", "a = 3"),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        let needing = steps_that_need_eyes(&chain, &[sub.clone()], |p, _| {
            p.extension().and_then(|e| e.to_str()) == Some("toml")
        });
        assert_eq!(needing, vec![0, 1]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_mkdir_outside_whitelist_is_marked() {
        let dir = tmp();
        let sub = dir.join("w");
        fs::create_dir_all(&sub).expect("mkdir");
        let inside = sub.join("ok");
        let outside = dir.join("elsewhere");
        let steps = vec![mkdir_step(&inside), mkdir_step(&outside)];
        let chain = play_forward(steps, &dir).expect("play");
        let needing = steps_that_need_eyes(&chain, &[sub.clone()], no_unapproved);
        assert_eq!(needing, vec![1]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn chmod_is_always_marked() {
        let dir = tmp();
        let sub = dir.join("w");
        fs::create_dir_all(&sub).expect("mkdir");
        let f = sub.join("a.sh");
        fs::write(&f, "x\n").expect("write");
        let steps = vec![chmod_step(&f)];
        let chain = play_forward(steps, &dir).expect("play");
        let needing = steps_that_need_eyes(&chain, &[sub.clone()], no_unapproved);
        assert_eq!(needing, vec![0]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn play_forward_leaves_review_indices_empty() {
        let dir = tmp();
        let file = dir.join("a.rs");
        fs::write(&file, "hello\n").expect("write");
        let steps = vec![text_step(Verb::Replace, &file, "hello", "world")];
        let chain = play_forward(steps, &dir).expect("play");
        assert!(chain.review_indices.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    // ---- hints (step 11) ----

    fn fail_text(steps: Vec<Step>, dir: &Path) -> String {
        match play_forward(steps, dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        }
    }

    #[test]
    fn typo_in_one_line_names_its_line_number() {
        let dir = tmp();
        let file = dir.join("a.rs");
        fs::write(
            &file,
            "fn a() {\n    let x = 1;\n    let result = compute(a, c);\n}\n",
        )
        .expect("write");
        let err = fail_text(
            vec![text_step(
                Verb::Replace,
                &file,
                "let result = compute(a, b);",
                "x",
            )],
            &dir,
        );
        assert!(err.contains("line 3 looks closest"), "got: {err}");
        assert!(err.contains("compute(a, c)"), "got: {err}");
        assert!(err.contains("found 0 ("), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn spaces_against_a_tab_give_the_whitespace_hint() {
        let dir = tmp();
        let file = dir.join("a.rs");
        fs::write(&file, "fn a() {\n    x();\n}\n").expect("write");
        let err = fail_text(
            vec![text_step(Verb::Replace, &file, "fn a() {\n\tx();\n}", "y")],
            &dir,
        );
        assert!(
            err.contains("matches at line 1 when indentation"),
            "got: {err}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn crlf_file_against_lf_old_text_gives_the_whitespace_hint() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "a\r\nb\r\nc\r\n").expect("write");
        let err = fail_text(vec![text_step(Verb::Replace, &file, "b\nc", "x")], &dir);
        assert!(
            err.contains("matches at line 2 when indentation"),
            "got: {err}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn three_line_old_text_names_the_differing_old_line() {
        let dir = tmp();
        let file = dir.join("a.rs");
        fs::write(&file, "fn a() {\n    let x = 1;\n}\n").expect("write");
        let err = fail_text(
            vec![text_step(
                Verb::Replace,
                &file,
                "fn a() {\n    let y = 2;\n}",
                "z",
            )],
            &dir,
        );
        assert!(
            err.contains("old-text line 2 differs from line 2"),
            "got: {err}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn nothing_similar_says_so_and_names_no_line() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\nworld\n").expect("write");
        let err = fail_text(
            vec![text_step(Verb::Replace, &file, "zzzz qqqq", "x")],
            &dir,
        );
        assert!(err.contains("no line in the file is close"), "got: {err}");
        assert!(!err.contains("looks closest"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn wrong_count_lists_the_match_lines() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "foo\nx\nfoo\nx\nfoo\n").expect("write");
        let err = fail_text(vec![text_step(Verb::Replace, &file, "foo", "bar")], &dir);
        assert!(err.contains("found 3 (at lines 1, 3, 5)"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn huge_file_gets_no_hint() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "x\n".repeat(25_000)).expect("write");
        let err = fail_text(vec![text_step(Verb::Replace, &file, "zzz", "y")], &dir);
        assert!(err.ends_with("found 0"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn create_on_existing_file_mentions_overwrite_and_lines() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "one\ntwo\n").expect("write");
        let err = fail_text(vec![text_step(Verb::Create, &file, "", "new\n")], &dir);
        assert!(err.contains("already exists ("), "got: {err}");
        assert!(err.contains("2 lines"), "got: {err}");
        assert!(err.contains("overwrite"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn overwrite_on_a_mistyped_name_suggests_the_sibling() {
        let dir = tmp();
        fs::write(dir.join("utils.rs"), "x\n").expect("write");
        let typo = dir.join("utlis.rs");
        let err = fail_text(vec![text_step(Verb::Overwrite, &typo, "", "y\n")], &dir);
        assert!(
            err.contains("does not exist (did you mean `utils.rs`?)"),
            "got: {err}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zaprm_on_a_mistyped_name_suggests_the_sibling() {
        let dir = tmp();
        fs::write(dir.join("utils.rs"), "x\n").expect("write");
        let err = fail_text(vec![rm_step(&dir.join("utlis.rs"))], &dir);
        assert!(err.contains("did you mean `utils.rs`"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_parent_folder_names_the_nearest_existing_one() {
        let dir = tmp();
        let file = dir.join("nope").join("x.rs");
        let err = fail_text(vec![text_step(Verb::Overwrite, &file, "", "y\n")], &dir);
        assert!(
            err.contains(&format!("the nearest existing folder is {}", dir.display())),
            "got: {err}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hints_keep_the_failures_on_one_line() {
        let dir = tmp();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "hello\n").expect("write");
        fs::write(&b, "world\n").expect("write");
        let err = fail_text(
            vec![
                text_step(Verb::Replace, &a, "nope", "x"),
                text_step(Verb::Replace, &b, "missing", "y"),
            ],
            &dir,
        );
        assert!(!err.contains('\n'), "got: {err}");
        assert!(
            err.contains("step 1 of 2") && err.contains("step 2 of 2"),
            "got: {err}"
        );
        assert!(err.contains("; step 2 of 2"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn suggest_name_rules() {
        let names = vec![
            "README.md".to_string(),
            "bat".to_string(),
            "cat".to_string(),
        ];
        assert_eq!(suggest_name("readme.md", &names), Some("README.md"));
        assert_eq!(suggest_name("aat", &names), Some("bat"));
        assert_eq!(suggest_name("zzzzzzzz", &names), None);
        assert_eq!(suggest_name("bat", &names), Some("cat"));
    }

    // ---- astgrep (step 13) ----

    fn astgrep_step(folder: &Path, old: &str, new: &str) -> Step {
        Step {
            verb: Verb::AstGrep,
            path: folder.display().to_string(),
            dest: None,
            old_text: old.to_string(),
            new_text: new.to_string(),
            count: None,
            no_newline: false,
            args: Vec::new(),
            cwd: None,
        }
    }

    /// A folder with one file under src/, and a fake ast-grep that reports
    /// every `word` in it. Returns (folder, file).
    fn astgrep_fixture(dir: &Path, text: &str, word: &str) -> (PathBuf, PathBuf) {
        let proj = dir.join("proj");
        fs::create_dir_all(proj.join("src")).expect("mkdir");
        let file = proj.join("src").join("a.rs");
        fs::write(&file, text).expect("write");
        let lines: String = text
            .match_indices(word)
            .map(|(i, w)| astgrep::testing::match_line(&file, i, i + w.len()))
            .collect();
        astgrep::testing::healthy(dir, &lines);
        (proj, file)
    }

    #[test]
    fn astgrep_plays_forward_as_one_step() {
        let dir = tmp();
        let (proj, file) =
            astgrep_fixture(&dir, "fn foo() {}\nfn main() { foo(); }\n", "foo");
        let chain = play_forward(vec![astgrep_step(&proj, "foo", "bar")], &dir).expect("play");
        assert_eq!(chain.steps.len(), 1);
        let s = &chain.steps[0];
        assert_eq!(s.path, proj);
        assert_eq!(s.files.len(), 1);
        assert_eq!(s.files[0].occurrences, 2);
        assert_eq!(s.files[0].after, "fn bar() {}\nfn main() { bar(); }\n");
        assert!(s.before.as_deref().unwrap_or("").contains(&file.display().to_string()));
        assert!(s.after.contains("fn bar() {}"));
        assert!(!s.rows.is_empty());
        assert!(!s.no_op);
        assert_eq!(chain.snapshots.len(), 1);
        assert_eq!(chain.boundary, proj.join("src"));
        assert!(chain.needs_review());
        assert!(chain.final_writes(&[]).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn astgrep_report_names_the_files() {
        let dir = tmp();
        let (proj, _file) =
            astgrep_fixture(&dir, "fn foo() {}\nfn main() { foo(); }\n", "foo");
        let chain = play_forward(vec![astgrep_step(&proj, "foo", "bar")], &dir).expect("play");
        let rep = crate::chain::report::build(&chain, &[crate::chain::StepStatus::Applied], None);
        assert!(rep.contains("renamed identifier `foo` to `bar`"), "got: {rep}");
        assert!(rep.contains("2 occurrences in 1 files"), "got: {rep}");
        assert!(rep.contains("src/a.rs"), "got: {rep}");
        let denied = crate::chain::report::build(&chain, &[crate::chain::StepStatus::Denied], None);
        assert!(denied.contains("would have renamed identifier"), "got: {denied}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn astgrep_with_no_matches_fails_the_step() {
        let dir = tmp();
        let (proj, _file) = astgrep_fixture(&dir, "fn foo() {}\n", "nothing_like_it");
        let err = fail_text(vec![astgrep_step(&proj, "foo", "bar")], &dir);
        assert!(err.contains("step 1 of 1 (astgrep"), "got: {err}");
        assert!(err.contains("no identifier named"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn astgrep_on_a_missing_folder_fails() {
        let dir = tmp();
        let err = fail_text(vec![astgrep_step(&dir.join("nope"), "foo", "bar")], &dir);
        assert!(err.contains("not a folder"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_edit_before_astgrep_to_a_file_under_its_folder_is_refused() {
        let dir = tmp();
        let (proj, file) = astgrep_fixture(&dir, "fn foo() {}\n// tail\n", "foo");
        let steps = vec![
            text_step(Verb::Replace, &file, "tail", "end"),
            astgrep_step(&proj, "foo", "bar"),
        ];
        let err = fail_text(steps, &dir);
        assert!(err.contains("step 1 (replace"), "got: {err}");
        assert!(
            err.contains("which astgrep rewrites, put it in a separate paste"),
            "got: {err}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_later_step_on_a_changed_file_is_refused_for_any_verb() {
        let dir = tmp();
        let (proj, file) = astgrep_fixture(&dir, "fn foo() {}\n", "foo");
        let steps = vec![
            astgrep_step(&proj, "foo", "bar"),
            mv_step(&file, &dir.join("b.rs")),
        ];
        let err = fail_text(steps, &dir);
        assert!(err.contains("step 2 (zapmv"), "got: {err}");
        assert!(err.contains("which astgrep rewrites"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn steps_on_other_files_and_mkdir_do_not_clash_with_astgrep() {
        let dir = tmp();
        let (proj, _file) = astgrep_fixture(&dir, "fn foo() {}\n", "foo");
        let notes = proj.join("notes.txt");
        fs::write(&notes, "x\n").expect("write");
        let steps = vec![
            mkdir_step(&proj.join("newdir")),
            text_step(Verb::Replace, &notes, "x", "y"),
            astgrep_step(&proj, "foo", "bar"),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        assert_eq!(chain.steps.len(), 3);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn collision_count_counts_whole_words_only() {
        let e = astgrep::FileEdit {
            path: PathBuf::from("/x/a.rs"),
            before: "let bar = 1; let barn = bar;\n".to_string(),
            after: String::new(),
            occurrences: 0,
        };
        assert_eq!(collision_count(&[e], "bar"), 2);
    }
}
