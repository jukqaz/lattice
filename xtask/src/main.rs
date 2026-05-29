use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("verify") => verify(),
        Some("linux-verify") => linux_verify(),
        Some("quality") => quality(),
        Some(command) => Err(format!("unknown xtask command: {command}")),
        None => Err("usage: cargo run -p xtask -- <verify|linux-verify|quality>".to_string()),
    }
}

fn linux_verify() -> Result<(), String> {
    let root = workspace_root();
    let root_str = root
        .to_str()
        .ok_or_else(|| format!("workspace path is not utf-8: {}", root.display()))?;
    let image = env::var("LATTICE_LINUX_IMAGE").unwrap_or_else(|_| "rust:1.95-bookworm".into());
    let mount = format!("{root_str}:/workspace:ro");
    let script = "set -euo pipefail; \
mkdir -p /tmp/lattice; \
tar --exclude=target --exclude=.git --exclude=.lattice-snapshots -cf - -C /workspace . | tar -xf - -C /tmp/lattice; \
cd /tmp/lattice; \
rustup component add rustfmt clippy; \
rustup target add wasm32-wasip2; \
cargo run -p xtask -- verify";

    run_passthrough(
        &root,
        "docker",
        [
            "run",
            "--rm",
            "-v",
            &mount,
            "-e",
            "CARGO_TERM_COLOR=never",
            &image,
            "bash",
            "-c",
            script,
        ],
        [],
    )?;
    println!("lattice xtask linux-verify: ok");
    Ok(())
}

fn quality() -> Result<(), String> {
    let root = workspace_root();
    ensure_required_tool("cargo-deny", &["--version"])?;
    ensure_required_tool("cargo-machete", &["--version"])?;
    ensure_required_tool("cargo", &["llvm-cov", "--version"])?;
    ensure_required_tool("typos", &["--version"])?;

    verify()?;
    run_passthrough(&root, "cargo-deny", ["check"], [])?;
    run_passthrough(
        &root,
        "cargo-machete",
        ["--with-metadata", "--skip-target-dir"],
        [],
    )?;
    run_passthrough(&root, "typos", ["--config", "_typos.toml"], [])?;
    run_passthrough(
        &root,
        "rustup",
        ["component", "add", "llvm-tools-preview"],
        [],
    )?;
    fs::create_dir_all(root.join("target/llvm-cov"))
        .map_err(|error| format!("failed to create target/llvm-cov: {error}"))?;
    run_passthrough(
        &root,
        "cargo",
        [
            "llvm-cov",
            "--workspace",
            "--all-features",
            "--locked",
            "--lcov",
            "--output-path",
            "target/llvm-cov/lcov.info",
        ],
        [],
    )?;

    println!("lattice xtask quality: ok");
    Ok(())
}

fn verify() -> Result<(), String> {
    let root = workspace_root();

    run_passthrough(&root, "cargo", ["fmt", "--check"], [])?;
    run_passthrough(
        &root,
        "cargo",
        ["clippy", "--workspace", "--all-targets", "--all-features"],
        [],
    )?;
    run_passthrough(&root, "cargo", ["test", "--workspace"], [])?;

    let temp = TempTree::new("lattice-harness")?;
    let xdg = XdgEnv::new(temp.path());

    run_capture(
        &root,
        "cargo",
        ["run", "--quiet", "--", "init", "--force"],
        &xdg,
    )?;
    ensure(
        mode(&xdg.config.join("lattice/lattice.toml"))? == 0o600,
        "global config mode mismatch",
    )?;
    ensure(
        xdg.config.join("lattice/services").is_dir(),
        "services directory missing after init",
    )?;
    run_capture(&root, "cargo", ["run", "--quiet", "--", "doctor"], &xdg)?;

    let services = run_capture(
        &root,
        "cargo",
        ["run", "--quiet", "--", "service", "list"],
        &xdg,
    )?;
    ensure(
        services.trim().is_empty(),
        "init should not create a tool-specific service",
    )?;

    let source = temp.path().join("shell-source");
    let repo = temp.path().join("shell-repo");
    write_file(&source, "config.toml", "prompt = \"compact\"\n", 0o600)?;
    write_file(&source, "bin/tool", "#!/usr/bin/env bash\n", 0o700)?;
    write_file(&source, "auth.json", "{}\n", 0o600)?;

    let service_config = format!(
        r#"name = "shell"
root = "{}"
repo = "{}"
include = ["config.toml", "bin/**"]
exclude = ["auth.json"]

[restore]
create_dirs = [
  {{ path = "cache", mode = "0700" }},
]

[[permissions]]
path = "config.toml"
mode = "0600"

[[permissions]]
path = "bin/tool"
mode = "0700"
"#,
        source.display(),
        repo.display()
    );
    fs::write(
        xdg.config.join("lattice/services/shell.toml"),
        service_config,
    )
    .map_err(|error| format!("failed to write service config: {error}"))?;

    let services = run_capture(
        &root,
        "cargo",
        ["run", "--quiet", "--", "service", "list"],
        &xdg,
    )?;
    ensure(
        services.lines().any(|line| line == "shell"),
        "service list did not include shell",
    )?;

    let status = run_capture(
        &root,
        "cargo",
        ["run", "--quiet", "--", "status", "shell"],
        &xdg,
    )?;
    ensure(
        status.contains("service: shell"),
        "status output did not include service name",
    )?;
    ensure(
        status.contains("included files: 2"),
        "status output did not include expected file count",
    )?;
    ensure(
        status.contains("manifest: missing"),
        "status output should report missing manifest before backup",
    )?;

    let dry_backup = run_capture(
        &root,
        "cargo",
        ["run", "--quiet", "--", "backup", "--dry-run", "shell"],
        &xdg,
    )?;
    ensure(
        dry_backup.contains("would copy 2 files"),
        "backup dry-run output did not include expected file count",
    )?;
    ensure(
        !repo.join("config.toml").exists(),
        "backup dry-run wrote config.toml",
    )?;
    ensure(
        !repo.join(".lattice/manifest.toml").exists(),
        "backup dry-run wrote manifest",
    )?;

    run_capture(
        &root,
        "cargo",
        ["run", "--quiet", "--", "backup", "shell"],
        &xdg,
    )?;
    ensure(
        repo.join("config.toml").is_file(),
        "repo missing config.toml",
    )?;
    ensure(repo.join("bin/tool").is_file(), "repo missing bin/tool")?;
    ensure(
        !repo.join("auth.json").exists(),
        "repo should not contain auth.json",
    )?;
    ensure(
        repo.join(".lattice/manifest.toml").is_file(),
        "repo missing manifest",
    )?;

    let dry_restore = run_capture(
        &root,
        "cargo",
        ["run", "--quiet", "--", "restore", "--dry-run", "shell"],
        &xdg,
    )?;
    ensure(
        dry_restore.contains("would restore 2 files"),
        "restore dry-run output did not include expected file count",
    )?;

    fs::remove_dir_all(&source)
        .map_err(|error| format!("failed to remove source tree before restore: {error}"))?;
    run_capture(
        &root,
        "cargo",
        ["run", "--quiet", "--", "restore", "shell"],
        &xdg,
    )?;

    ensure(
        source.join("config.toml").is_file(),
        "restore missing config.toml",
    )?;
    ensure(
        source.join("bin/tool").is_file(),
        "restore missing bin/tool",
    )?;
    ensure(source.join("cache").is_dir(), "restore missing cache")?;
    ensure(
        mode(&source.join("config.toml"))? == 0o600,
        "config.toml mode mismatch",
    )?;
    ensure(
        mode(&source.join("bin/tool"))? == 0o700,
        "bin/tool mode mismatch",
    )?;
    ensure(mode(&source.join("cache"))? == 0o700, "cache mode mismatch")?;

    verify_product_surface_harness(&root)?;
    verify_cli_edge_harness(&root)?;
    verify_non_unix_compile_harness(&root)?;

    println!("lattice xtask verify: ok");
    Ok(())
}

fn ensure_required_tool(tool: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(tool)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| {
            format!(
                "required quality tool {tool} {args:?} is not installed or not executable: {error}. \
Install with cargo install cargo-deny cargo-machete cargo-llvm-cov typos-cli --locked"
            )
        })?;
    ensure(
        status.success(),
        &format!("required quality tool {tool} {args:?} exited with {status}"),
    )
}

fn verify_product_surface_harness(root: &Path) -> Result<(), String> {
    let temp = TempTree::new("lattice-product-surface")?;
    let xdg = XdgEnv::new(temp.path());

    let top_help = run_capture(root, "cargo", ["run", "--quiet", "--", "--help"], &xdg)?;
    ensure_contains(
        &top_help,
        "app",
        "top-level help should expose app commands",
    )?;
    ensure_contains(
        &top_help,
        "bootstrap",
        "top-level help should expose bootstrap commands",
    )?;
    ensure_contains(&top_help, "plan", "top-level help should expose plan")?;
    ensure_contains(&top_help, "group", "top-level help should expose groups")?;
    ensure_contains(
        &top_help,
        "Inspect service groups without mutating state",
        "top-level help should describe groups as read-only inspection",
    )?;
    ensure_not_contains_case_insensitive(
        &top_help,
        "preset",
        "top-level help should not expose old catalog terminology",
    )?;

    let group_help = run_capture(
        root,
        "cargo",
        ["run", "--quiet", "--", "group", "--help"],
        &xdg,
    )?;
    for needle in [
        "list",
        "show",
        "status",
        "plan",
        "List configured service groups",
        "Show one service group",
        "Show grouped service status",
        "Summarize grouped service plans",
    ] {
        ensure_contains(&group_help, needle, &format!("group help missing {needle}"))?;
    }
    ensure_not_contains_case_insensitive(
        &group_help,
        "backup",
        "group help should not expose batch backup mutation",
    )?;
    ensure_not_contains_case_insensitive(
        &group_help,
        "restore",
        "group help should not expose batch restore mutation",
    )?;
    for mutation in ["backup", "restore"] {
        let error = run_capture_fail(
            root,
            "cargo",
            ["run", "--quiet", "--", "group", mutation, "dev-shell"],
            &xdg,
        )?;
        ensure_contains(
            &error,
            "unrecognized subcommand",
            &format!("group {mutation} should be rejected as unsupported batch mutation"),
        )?;
    }

    let app_help = run_capture(
        root,
        "cargo",
        ["run", "--quiet", "--", "app", "--help"],
        &xdg,
    )?;
    ensure_contains(&app_help, "list", "app help should expose list")?;
    ensure_contains(&app_help, "show", "app help should expose show")?;
    ensure_contains(&app_help, "add", "app help should expose add")?;
    ensure_not_contains_case_insensitive(
        &app_help,
        "preset",
        "app help should not preserve old catalog terminology",
    )?;

    let service_add_help = run_capture(
        root,
        "cargo",
        ["run", "--quiet", "--", "service", "add", "--help"],
        &xdg,
    )?;
    ensure_not_contains_case_insensitive(
        &service_add_help,
        "preset",
        "service add help should not expose old catalog flags",
    )?;

    for relative in [
        "README.md",
        "README.ko.md",
        "TODO.md",
        "docs/product/mvp-scope.md",
        "docs/product/mvp-scope.ko.md",
        "docs/user/usage.md",
        "docs/user/usage.ko.md",
        "docs/reference/json-output.md",
        "docs/reference/json-output.ko.md",
        "docs/dev/quality.md",
        "docs/dev/quality.ko.md",
    ] {
        let body = read_repo_text(root, relative)?;
        ensure_not_contains_case_insensitive(
            &body,
            "preset",
            &format!("{relative} should use app/service catalog wording, not preset wording"),
        )?;
    }

    let readme = read_repo_text(root, "README.md")?;
    for needle in [
        "lattice app list",
        "lattice app show",
        "lattice app add",
        "lattice bootstrap check",
        "lattice plan",
        "Apps are not product centers",
        "lattice group list --json",
        "lattice group show --json",
        "lattice group status --json",
        "lattice group plan --json",
        "Service Groups",
        "Group commands are intentionally read-only in v0.5",
        "There is no `group backup` or `group restore` yet",
        "conflict_count",
        "active=false",
        "Safe first-adoption playbook",
        "Do not run restore first on a real HOME",
        "--tag v0.5.1",
        "beyond the v0.5.1 release",
        "next_actions",
    ] {
        ensure_contains(&readme, needle, &format!("README.md missing {needle}"))?;
    }

    let korean_readme = read_repo_text(root, "README.ko.md")?;
    for needle in [
        "lattice group list --json",
        "lattice group show --json",
        "lattice group status --json",
        "lattice group plan --json",
        "Service Groups",
        "conflict_count",
        "active=false",
        "--tag v0.5.1",
        "v0.5.1 release 이후",
    ] {
        ensure_contains(
            &korean_readme,
            needle,
            &format!("README.ko.md missing {needle}"),
        )?;
    }

    let user_guide = read_repo_text(root, "docs/user/usage.md")?;
    for needle in [
        "lattice app list",
        "lattice app show",
        "lattice app add",
        "lattice bootstrap check",
        "lattice plan --json",
        "Codex is\nonly one example app",
        "lattice group list --json",
        "lattice group show --json",
        "lattice group status --json",
        "lattice group plan --json",
        "Selector",
        "Safe first-adoption playbook",
        "Do not run restore first on a real HOME",
        "--tag v0.5.1",
        "beyond the v0.5.1 release",
        "next_command",
        "next_actions",
    ] {
        ensure_contains(
            &user_guide,
            needle,
            &format!("docs/user/usage.md missing {needle}"),
        )?;
    }

    let korean_user_guide = read_repo_text(root, "docs/user/usage.ko.md")?;
    for needle in [
        "lattice group list --json",
        "lattice group show --json",
        "lattice group status --json",
        "lattice group plan --json",
        "Selector",
        "batch backup",
        "conflict_count",
        "active=false",
        "--tag v0.5.1",
        "v0.5.1 release 이후",
    ] {
        ensure_contains(
            &korean_user_guide,
            needle,
            &format!("docs/user/usage.ko.md missing {needle}"),
        )?;
    }

    let product_scope = read_repo_text(root, "docs/product/mvp-scope.md")?;
    for needle in [
        "Codex is one example app",
        "`plan` as the single human/JSON preflight surface",
        "`bootstrap check` for new-machine readiness diagnostics",
        "`app list`, `app show <app>`, and `app add <app>`",
        "product-surface harness coverage",
        "Service Groups",
        "`group list/show/status/plan`",
        "JSON output",
        "group invariant validation",
        "active-only aggregates",
        "missing-root visibility",
        "v0.5.1 hardens the service-groups release line",
        "v0.5.1 scope",
    ] {
        ensure_contains(
            &product_scope,
            needle,
            &format!("docs/product/mvp-scope.md missing {needle}"),
        )?;
    }

    let korean_scope = read_repo_text(root, "docs/product/mvp-scope.ko.md")?;
    for needle in [
        "Codex는 제품의 중심이 아니라 예시 앱 중 하나",
        "`plan`",
        "`bootstrap check`",
        "`app list`, `app show <app>`, `app add <app>`",
        "product-surface harness",
        "Service Groups",
        "`group list/show/status/plan`",
        "JSON output",
        "group invariant validation",
        "active-only aggregate",
        "missing-root visibility",
        "v0.5.1은 service-groups release line을 harden한다",
        "v0.5.1 범위",
    ] {
        ensure_contains(
            &korean_scope,
            needle,
            &format!("docs/product/mvp-scope.ko.md missing {needle}"),
        )?;
    }

    let docs_index = read_repo_text(root, "docs/README.md")?;
    for needle in [
        "reference/json-output.md",
        "dev/quality.md",
        "reference/json-output.ko.md",
        "dev/quality.ko.md",
        "llm/kanban-workflow.md",
    ] {
        ensure_contains(
            &docs_index,
            needle,
            &format!("docs/README.md missing {needle}"),
        )?;
    }

    let llm_index = read_repo_text(root, "docs/llm/README.md")?;
    for needle in [
        "docs/llm/kanban-workflow.md",
        "one active card at a time",
        "Hermes Kanban recovery board",
    ] {
        ensure_contains(
            &llm_index,
            needle,
            &format!("docs/llm/README.md missing {needle}"),
        )?;
    }

    let kanban_workflow = read_repo_text(root, "docs/llm/kanban-workflow.md")?;
    for needle in [
        "# Lattice Kanban Workflow For LLM Agents",
        "one active card at a time",
        "Do not run mutating live HOME commands",
        "complete the current card before unblocking exactly the next card",
        "recovery board",
        "cargo run -p xtask -- verify",
        "git diff --check",
    ] {
        ensure_contains(
            &kanban_workflow,
            needle,
            &format!("docs/llm/kanban-workflow.md missing {needle}"),
        )?;
    }

    let json_reference = read_repo_text(root, "docs/reference/json-output.md")?;
    for needle in [
        "lattice group list --json",
        "lattice group status --json",
        "root_exists=null",
        "conflict_count",
        "next_command",
        "next_actions",
        "There is no `group backup` or `group restore` in v0.5",
    ] {
        ensure_contains(
            &json_reference,
            needle,
            &format!("docs/reference/json-output.md missing {needle}"),
        )?;
    }

    let quality_doc = read_repo_text(root, "docs/dev/quality.md")?;
    for needle in [
        "cargo run -p xtask -- verify",
        "cargo install cargo-deny --locked",
        "cargo install cargo-machete --locked",
        "cargo install cargo-llvm-cov --locked",
        "cargo install typos-cli --locked",
        "cargo run -p xtask -- quality",
        "scripts/real-home-readonly-health-check.sh",
        "read-only real HOME health check",
    ] {
        ensure_contains(
            &quality_doc,
            needle,
            &format!("docs/dev/quality.md missing {needle}"),
        )?;
    }

    let home_check_script = read_repo_text(root, "scripts/real-home-readonly-health-check.sh")?;
    for needle in [
        "lattice real HOME read-only health check",
        "READ_ONLY_COMMANDS",
        "doctor",
        "validate",
        "bootstrap check",
        "service list",
        "discover --json",
        "group list --json",
        "MUTATING_COMMANDS",
        "init",
        "backup",
        "restore",
    ] {
        ensure_contains(
            &home_check_script,
            needle,
            &format!("scripts/real-home-readonly-health-check.sh missing {needle}"),
        )?;
    }

    verify_real_home_readonly_script(root)?;

    let changelog = read_repo_text(root, "CHANGELOG.md")?;
    for needle in [
        "## Unreleased",
        "discover` now includes per-suggestion `next_command` hints",
        "## v0.5.1 - 2026-05-26",
        "discover` now reports suggestion-level warnings",
        "Do not run restore first on a real HOME",
        "wasm32-wasip2",
    ] {
        ensure_contains(
            &changelog,
            needle,
            &format!("CHANGELOG.md missing {needle}"),
        )?;
    }

    let korean_changelog = read_repo_text(root, "CHANGELOG.ko.md")?;
    for needle in [
        "## Unreleased",
        "top-level `next_actions`",
        "## v0.5.1 - 2026-05-26",
        "suggestion-level warning",
        "real HOME",
        "wasm32-wasip2",
    ] {
        ensure_contains(
            &korean_changelog,
            needle,
            &format!("CHANGELOG.ko.md missing {needle}"),
        )?;
    }

    let ci_workflow = read_repo_text(root, ".github/workflows/ci.yml")?;
    for needle in [
        "--target wasm32-wasip2",
        "name: Check non-Unix core compile",
        "cargo check -p lattice-core --target wasm32-wasip2",
    ] {
        ensure_contains(
            &ci_workflow,
            needle,
            &format!(".github/workflows/ci.yml missing {needle}"),
        )?;
    }
    let non_unix_check_count = ci_workflow
        .matches("cargo check -p lattice-core --target wasm32-wasip2")
        .count();
    ensure(
        non_unix_check_count == 1,
        &format!(
            ".github/workflows/ci.yml should contain exactly one non-Unix core compile check; found {non_unix_check_count}"
        ),
    )?;

    Ok(())
}

fn verify_cli_edge_harness(root: &Path) -> Result<(), String> {
    let temp = TempTree::new("lattice-edge-harness")?;
    let xdg = XdgEnv::new(temp.path());
    run_capture(
        root,
        "cargo",
        ["run", "--quiet", "--", "init", "--force"],
        &xdg,
    )?;

    let mismatch_source = temp.path().join("mismatch-source");
    write_file(
        &mismatch_source,
        "settings.toml",
        "theme = \"dark\"\n",
        0o600,
    )?;
    let mismatch_service = format!(
        r#"name = "different"
root = "{}"
include = ["settings.toml"]
"#,
        mismatch_source.display()
    );
    write_file(
        &xdg.config.join("lattice/services"),
        "mismatch.toml",
        &mismatch_service,
        0o600,
    )?;
    let mismatch_error =
        run_capture_fail(root, "cargo", ["run", "--quiet", "--", "validate"], &xdg)?;
    ensure(
        mismatch_error.contains("service config name mismatch"),
        "service name mismatch did not fail validation",
    )?;
    fs::remove_file(xdg.config.join("lattice/services/mismatch.toml"))
        .map_err(|error| format!("failed to remove mismatched service config: {error}"))?;

    let tui_error = run_capture_fail(root, "cargo", ["run", "--quiet", "--", "tui"], &xdg)?;
    ensure(
        tui_error.contains("interactive TUI requires a terminal"),
        "noninteractive TUI did not fail with terminal guidance",
    )?;

    fs::write(xdg.config.join("lattice/lattice.toml"), "version =\n")
        .map_err(|error| format!("failed to write invalid global config: {error}"))?;
    let validate_error =
        run_capture_fail(root, "cargo", ["run", "--quiet", "--", "validate"], &xdg)?;
    ensure(
        validate_error.contains("failed to parse"),
        "invalid config did not fail validate",
    )?;
    run_capture(
        root,
        "cargo",
        ["run", "--quiet", "--", "init", "--force"],
        &xdg,
    )?;

    let source = temp.path().join("edge-source");
    write_file(&source, "settings.toml", "theme = \"dark\"\n", 0o600)?;
    let source_str = source
        .to_str()
        .ok_or_else(|| "edge source path is not utf-8".to_string())?;
    run_capture(
        root,
        "cargo",
        [
            "run",
            "--quiet",
            "--",
            "service",
            "add",
            "edge",
            "--root",
            source_str,
            "--include",
            "settings.toml",
        ],
        &xdg,
    )?;
    ensure(
        mode(&xdg.config.join("lattice/services/edge.toml"))? == 0o600,
        "edge service config mode mismatch",
    )?;
    let permission_error = run_capture_fail(
        root,
        "cargo",
        [
            "run",
            "--quiet",
            "--",
            "permission",
            "set",
            "edge",
            "../secret.txt",
            "0600",
        ],
        &xdg,
    )?;
    ensure(
        permission_error.contains("unsafe relative path"),
        "unsafe permission path did not fail",
    )?;
    let service_config = run_capture(
        root,
        "cargo",
        ["run", "--quiet", "--", "service", "show", "edge"],
        &xdg,
    )?;
    ensure(
        !service_config.contains("../secret.txt"),
        "unsafe permission path was persisted",
    )?;
    write_file(
        &source,
        "secret.env",
        &format!(
            "OPENAI_API_KEY={}proj_fake_but_token_shaped\n",
            ["s", "k-"].concat()
        ),
        0o600,
    )?;
    let adopt_error = run_capture_fail(
        root,
        "cargo",
        ["run", "--quiet", "--", "adopt", "edge", "secret.env"],
        &xdg,
    )?;
    ensure(
        adopt_error.contains("secret-looking content"),
        "failed adopt did not report secret-looking content",
    )?;
    let service_config = run_capture(
        root,
        "cargo",
        ["run", "--quiet", "--", "service", "show", "edge"],
        &xdg,
    )?;
    ensure(
        !service_config.contains("secret.env"),
        "failed adopt persisted secret.env tracking",
    )?;
    run_capture(
        root,
        "cargo",
        ["run", "--quiet", "--", "backup", "edge"],
        &xdg,
    )?;
    let repo = xdg.data.join("lattice/repos/edge");
    ensure(
        !repo.join("secret.env").exists(),
        "failed adopt copied secret.env into repo",
    )?;
    let repo_status_error = run_capture_fail(
        root,
        "cargo",
        ["run", "--quiet", "--", "repo", "status", "edge"],
        &xdg,
    )?;
    ensure(
        repo_status_error.contains("repo is not a git repository"),
        "repo status did not reject non-git repo",
    )?;
    run_git(&repo, ["init"])?;
    run_git(&repo, ["config", "user.email", "lattice@example.test"])?;
    run_git(&repo, ["config", "user.name", "Lattice Test"])?;
    let push_error = run_capture_fail(
        root,
        "cargo",
        ["run", "--quiet", "--", "repo", "push", "edge"],
        &xdg,
    )?;
    ensure(
        push_error.contains("git exited"),
        "repo push without remote did not surface git failure",
    )?;
    fs::write(
        repo.join("leak.env"),
        format!(
            "OPENAI_API_KEY={}proj_fake_but_token_shaped\n",
            ["s", "k-"].concat()
        ),
    )
    .map_err(|error| format!("failed to write repo leak: {error}"))?;
    let commit_error = run_capture_fail(
        root,
        "cargo",
        [
            "run",
            "--quiet",
            "--",
            "repo",
            "commit",
            "edge",
            "--message",
            "backup configs",
        ],
        &xdg,
    )?;
    ensure(
        commit_error.contains("secret-looking content"),
        "repo commit did not block secret-looking content",
    )?;

    let binary_source = temp.path().join("binary-source");
    let binary_repo = temp.path().join("binary-repo");
    fs::create_dir_all(&binary_source)
        .map_err(|error| format!("failed to create binary source: {error}"))?;
    write_bytes(&binary_source.join("blob.bin"), &[0, 159, 146, 150])?;
    let binary_service = format!(
        r#"name = "binary"
root = "{}"
repo = "{}"
include = ["blob.bin"]
"#,
        binary_source.display(),
        binary_repo.display()
    );
    write_file(
        &xdg.config.join("lattice/services"),
        "binary.toml",
        &binary_service,
        0o600,
    )?;
    run_capture(
        root,
        "cargo",
        ["run", "--quiet", "--", "backup", "binary"],
        &xdg,
    )?;
    write_bytes(&binary_source.join("blob.bin"), &[0, 1, 2, 3, 255])?;
    let diff = run_capture(
        root,
        "cargo",
        ["run", "--quiet", "--", "diff", "binary"],
        &xdg,
    )?;
    ensure(
        diff.contains("binary content differs; line diff hidden"),
        "binary diff did not hide line output",
    )?;

    Ok(())
}

fn verify_non_unix_compile_harness(root: &Path) -> Result<(), String> {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map_err(|error| format!("failed to list rustup targets: {error}"))?;
    ensure(output.status.success(), "rustup target list failed")?;
    let installed = String::from_utf8_lossy(&output.stdout);
    if installed.lines().any(|line| line == "wasm32-wasip2") {
        run_passthrough(
            root,
            "cargo",
            ["check", "-p", "lattice-core", "--target", "wasm32-wasip2"],
            [],
        )?;
    } else {
        println!(
            "lattice xtask verify: skipped non-unix compile check; wasm32-wasip2 target is not installed"
        );
    }
    Ok(())
}

fn verify_real_home_readonly_script(root: &Path) -> Result<(), String> {
    let path = root.join("scripts/real-home-readonly-health-check.sh");
    let script = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    for needle in [
        "set -euo pipefail",
        "lattice real HOME read-only health check",
        "READ_ONLY_COMMANDS",
        "MUTATING_COMMANDS",
        "lattice doctor",
        "lattice validate",
        "lattice bootstrap check",
        "lattice service list",
        "lattice status --json",
        "lattice plan --json",
        "lattice discover --json",
        "lattice group list --json",
        "lattice group status --json",
        "lattice group plan --json",
        "LAST_STATUS",
        "if [[ ${status} -ne 0 ]]",
    ] {
        ensure_contains(
            &script,
            needle,
            &format!("scripts/real-home-readonly-health-check.sh missing {needle}"),
        )?;
    }

    for forbidden in [
        "run_lattice init",
        "run_lattice backup",
        "run_lattice restore",
        "run_lattice adopt",
        "run_lattice undo",
        "run_lattice snapshot prune",
        "run_lattice service add",
        "run_lattice service remove",
        "run_lattice include add",
        "run_lattice include remove",
        "run_lattice exclude add",
        "run_lattice exclude remove",
        "run_lattice permission set",
        "run_lattice permission remove",
        "run_lattice repo commit",
        "run_lattice repo push",
        "run_lattice repo pull",
        "cargo run --quiet",
        "cargo run",
    ] {
        ensure_not_contains_case_insensitive(
            &script,
            forbidden,
            &format!("read-only health script must not run {forbidden}"),
        )?;
    }

    #[cfg(unix)]
    ensure(
        mode(&path)? & 0o111 != 0,
        "scripts/real-home-readonly-health-check.sh must be executable",
    )?;

    Ok(())
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn run_passthrough<const N: usize, const M: usize>(
    root: &Path,
    program: &str,
    args: [&str; N],
    envs: [(&str, &Path); M],
) -> Result<(), String> {
    let status = base_command(root, program, args, envs)
        .status()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    ensure(status.success(), &format!("{program} exited with {status}"))
}

fn run_capture<const N: usize>(
    root: &Path,
    program: &str,
    args: [&str; N],
    xdg: &XdgEnv,
) -> Result<String, String> {
    let output = base_command(root, program, args, xdg.envs())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("failed to run {program}: {error}"))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    Err(format!(
        "{program} exited with {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn run_capture_fail<const N: usize>(
    root: &Path,
    program: &str,
    args: [&str; N],
    xdg: &XdgEnv,
) -> Result<String, String> {
    let output = base_command(root, program, args, xdg.envs())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("failed to run {program}: {error}"))?;

    if output.status.success() {
        return Err(format!(
            "{program} unexpectedly succeeded for args {args:?}"
        ));
    }

    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn base_command<const N: usize, const M: usize>(
    root: &Path,
    program: &str,
    args: [&str; N],
    envs: [(&str, &Path); M],
) -> Command {
    let mut command = Command::new(program);
    command.current_dir(root).args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command
}

fn write_file(root: &Path, relative: &str, body: &str, mode: u32) -> Result<(), String> {
    let path = root.join(relative);
    let parent = path
        .parent()
        .ok_or_else(|| format!("{relative} has no parent"))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    fs::write(&path, body)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    set_mode(&path, mode)
}

fn write_bytes(path: &Path, body: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    fs::write(path, body).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn run_git<const N: usize>(repo: &Path, args: [&str; N]) -> Result<(), String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run git: {error}"))?;
    ensure(
        output.status.success(),
        &format!(
            "git failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(|error| format!("failed to chmod {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn mode(path: &Path) -> Result<u32, String> {
    use std::os::unix::fs::PermissionsExt;

    fs::metadata(path)
        .map_err(|error| format!("failed to stat {}: {error}", path.display()))
        .map(|metadata| metadata.permissions().mode() & 0o777)
}

#[cfg(not(unix))]
fn mode(_path: &Path) -> Result<u32, String> {
    Ok(0)
}

fn read_repo_text(root: &Path, relative: &str) -> Result<String, String> {
    let path = root.join(relative);
    fs::read_to_string(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn ensure_contains(body: &str, needle: &str, message: &str) -> Result<(), String> {
    ensure(body.contains(needle), message)
}

fn ensure_not_contains_case_insensitive(
    body: &str,
    needle: &str,
    message: &str,
) -> Result<(), String> {
    ensure(
        !body.to_lowercase().contains(&needle.to_lowercase()),
        message,
    )
}

fn ensure(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_string())
    }
}

struct XdgEnv {
    config: PathBuf,
    data: PathBuf,
    state: PathBuf,
    cache: PathBuf,
}

impl XdgEnv {
    fn new(root: &Path) -> Self {
        Self {
            config: root.join("config"),
            data: root.join("data"),
            state: root.join("state"),
            cache: root.join("cache"),
        }
    }

    fn envs(&self) -> [(&str, &Path); 4] {
        [
            ("XDG_CONFIG_HOME", self.config.as_path()),
            ("XDG_DATA_HOME", self.data.as_path()),
            ("XDG_STATE_HOME", self.state.as_path()),
            ("XDG_CACHE_HOME", self.cache.as_path()),
        ]
    }
}

struct TempTree {
    path: PathBuf,
}

impl TempTree {
    fn new(prefix: &str) -> Result<Self, String> {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock before epoch: {error}"))?
            .as_millis();
        let path = env::temp_dir().join(format!("{prefix}-{}-{millis}", std::process::id()));
        fs::create_dir_all(&path)
            .map_err(|error| format!("failed to create temp tree {}: {error}", path.display()))?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        if self
            .path
            .file_name()
            .and_then(OsStr::to_str)
            .is_some_and(|name| {
                name.starts_with("lattice-") && self.path.starts_with(env::temp_dir())
            })
        {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_home_readonly_health_check_script_has_static_safety_contract() {
        verify_real_home_readonly_script(&workspace_root()).unwrap();
    }
}
