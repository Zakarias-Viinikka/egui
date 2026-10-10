//! The cargo allow-list. zapgui consults this before starting a run.
//! zapcli consults it first for quick feedback; zapgui's check is the one
//! that counts.
//!
//! Nothing here is wired up yet; it exists so the check has one home and
//! can be unit-tested early.

const ALLOWED_SUBCOMMANDS: &[&str] = &[
    "build", "check", "run", "test", "bench", "clippy", "doc", "clean",
    "tree", "version",
    "b", "c", "d", "r", "t",
    "fmt",
];

const REFUSED_FLAGS: &[&str] = &[
    "--config",
    "--manifest-path",
    "--target-dir",
];

/// Check one cargo command line, without the leading `cargo`. `args` is the
/// argv after `cargo`. Returns Ok if the command may be started.
pub fn check(args: &[&str]) -> Result<(), String> {
    let mut i = 0;
    if let Some(first) = args.first() {
        if let Some(tc) = first.strip_prefix('+') {
            if !matches!(tc, "stable" | "beta" | "nightly") {
                return Err(format!("cargo: toolchain {tc:?} not allowed"));
            }
            i = 1;
        }
    }

    let Some(sub) = args.get(i) else {
        return Err("cargo: missing subcommand".to_string());
    };
    if !ALLOWED_SUBCOMMANDS.contains(sub) {
        return Err(format!("cargo: {sub:?} is not allowed"));
    }
    if *sub == "fmt" && !args.iter().any(|a| *a == "--check") {
        return Err("cargo fmt: only allowed with --check".to_string());
    }

    let mut after_dd = false;
    for a in &args[i + 1..] {
        if *a == "--" {
            after_dd = true;
            continue;
        }
        if after_dd {
            continue;
        }
        if let Some(tc) = a.strip_prefix('+') {
            if !matches!(tc, "stable" | "beta" | "nightly") {
                return Err(format!("cargo: toolchain {tc:?} not allowed"));
            }
        }
        if a.starts_with("-Z") {
            return Err("cargo: -Z flags not allowed".to_string());
        }
        for bad in REFUSED_FLAGS {
            if *a == *bad {
                return Err(format!("cargo: {bad} not allowed"));
            }
            let prefix = format!("{bad}=");
            if a.starts_with(&prefix) {
                return Err(format!("cargo: {bad} not allowed"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check;

    #[test]
    fn allows_build() {
        assert!(check(&["build"]).is_ok());
        assert!(check(&["build", "--release"]).is_ok());
    }

    #[test]
    fn allows_short_forms() {
        assert!(check(&["b"]).is_ok());
        assert!(check(&["c"]).is_ok());
        assert!(check(&["r", "--", "--help"]).is_ok());
    }

    #[test]
    fn refuses_install() {
        assert!(check(&["install", "cargo-edit"]).is_err());
    }

    #[test]
    fn refuses_publish() {
        assert!(check(&["publish"]).is_err());
    }

    #[test]
    fn refuses_manifest_path() {
        assert!(check(&["build", "--manifest-path", "/tmp/Cargo.toml"]).is_err());
        assert!(check(&["build", "--manifest-path=/tmp/Cargo.toml"]).is_err());
    }

    #[test]
    fn refuses_target_dir() {
        assert!(check(&["build", "--target-dir", "/tmp/t"]).is_err());
    }

    #[test]
    fn refuses_z_flags() {
        assert!(check(&["build", "-Zbuild-std"]).is_err());
    }

    #[test]
    fn refuses_other_toolchain() {
        assert!(check(&["+1.70.0", "build"]).is_err());
    }

    #[test]
    fn allows_stable_toolchain() {
        assert!(check(&["+stable", "build"]).is_ok());
    }

    #[test]
    fn fmt_only_with_check() {
        assert!(check(&["fmt"]).is_err());
        assert!(check(&["fmt", "--check"]).is_ok());
    }

    #[test]
    fn args_after_dd_are_not_inspected() {
        assert!(check(&["run", "--", "--manifest-path", "/tmp/Cargo.toml"]).is_ok());
        assert!(check(&["run", "--", "-Zsomething"]).is_ok());
    }
}
