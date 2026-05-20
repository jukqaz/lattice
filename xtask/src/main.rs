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
        Some(command) => Err(format!("unknown xtask command: {command}")),
        None => Err("usage: cargo run -p xtask -- verify".to_string()),
    }
}

fn verify() -> Result<(), String> {
    let root = workspace_root();

    run_passthrough(&root, "cargo", ["fmt", "--check"], [])?;
    run_passthrough(&root, "cargo", ["test"], [])?;

    let temp = TempTree::new("lattice-harness")?;
    let xdg = XdgEnv::new(temp.path());

    run_capture(
        &root,
        "cargo",
        ["run", "--quiet", "--", "init", "--force"],
        &xdg,
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

    println!("lattice xtask verify: ok");
    Ok(())
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask manifest should have a parent")
        .to_path_buf()
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
                name.starts_with("lattice-harness-") && self.path.starts_with(env::temp_dir())
            })
        {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
