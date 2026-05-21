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
        mode(&xdg.config.join("lattice/services/codex.toml"))? == 0o600,
        "codex service config mode mismatch",
    )?;
    run_capture(&root, "cargo", ["run", "--quiet", "--", "doctor"], &xdg)?;

    let services = run_capture(
        &root,
        "cargo",
        ["run", "--quiet", "--", "service", "list"],
        &xdg,
    )?;
    ensure(
        services.lines().any(|line| line == "codex"),
        "service list did not include codex",
    )?;

    let source = temp.path().join("codex-source");
    let repo = temp.path().join("codex-repo");
    write_file(&source, "config.toml", "model = \"gpt-5.5\"\n", 0o600)?;
    write_file(&source, "bin/mcp-rbw", "#!/usr/bin/env bash\n", 0o700)?;
    write_file(&source, "auth.json", "{}\n", 0o600)?;

    let service_config = format!(
        r#"name = "codex"
root = "{}"
repo = "{}"
preset = "codex"

[restore]
create_dirs = [
  {{ path = "shell_snapshots", mode = "0700" }},
]

[[permissions]]
path = "config.toml"
mode = "0600"
"#,
        source.display(),
        repo.display()
    );
    fs::write(
        xdg.config.join("lattice/services/codex.toml"),
        service_config,
    )
    .map_err(|error| format!("failed to write service config: {error}"))?;

    let status = run_capture(
        &root,
        "cargo",
        ["run", "--quiet", "--", "status", "codex"],
        &xdg,
    )?;
    ensure(
        status.contains("service: codex"),
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
        ["run", "--quiet", "--", "backup", "--dry-run", "codex"],
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
        ["run", "--quiet", "--", "backup", "codex"],
        &xdg,
    )?;
    ensure(
        repo.join("config.toml").is_file(),
        "repo missing config.toml",
    )?;
    ensure(
        repo.join("bin/mcp-rbw").is_file(),
        "repo missing bin/mcp-rbw",
    )?;
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
        ["run", "--quiet", "--", "restore", "--dry-run", "codex"],
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
        ["run", "--quiet", "--", "restore", "codex"],
        &xdg,
    )?;

    ensure(
        source.join("config.toml").is_file(),
        "restore missing config.toml",
    )?;
    ensure(
        source.join("bin/mcp-rbw").is_file(),
        "restore missing bin/mcp-rbw",
    )?;
    ensure(
        source.join("shell_snapshots").is_dir(),
        "restore missing shell_snapshots",
    )?;
    ensure(
        mode(&source.join("config.toml"))? == 0o600,
        "config.toml mode mismatch",
    )?;
    ensure(
        mode(&source.join("bin/mcp-rbw"))? == 0o700,
        "bin/mcp-rbw mode mismatch",
    )?;
    ensure(
        mode(&source.join("shell_snapshots"))? == 0o700,
        "shell_snapshots mode mismatch",
    )?;

    verify_real_codex_dry_run(&root)?;
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

fn verify_real_codex_dry_run(root: &Path) -> Result<(), String> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    let codex_home = home.join(".codex");
    if !codex_home.is_dir() {
        return Ok(());
    }

    let temp = TempTree::new("lattice-real-codex-dry-run")?;
    let xdg = XdgEnv::new(temp.path());
    run_capture(
        root,
        "cargo",
        ["run", "--quiet", "--", "init", "--force"],
        &xdg,
    )?;
    let status = run_capture(
        root,
        "cargo",
        ["run", "--quiet", "--", "status", "codex"],
        &xdg,
    )?;
    ensure(
        status.contains("service: codex"),
        "real codex status did not include service",
    )?;
    let dry_run = run_capture(
        root,
        "cargo",
        ["run", "--quiet", "--", "backup", "--dry-run", "codex"],
        &xdg,
    )?;
    ensure(
        dry_run.contains("would copy"),
        "real codex dry-run did not report copy plan",
    )?;
    ensure(
        !xdg.data
            .join("lattice/repos/codex/.lattice/manifest.toml")
            .exists(),
        "real codex dry-run wrote a manifest",
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
