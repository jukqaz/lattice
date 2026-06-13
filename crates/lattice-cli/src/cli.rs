use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "lattice")]
#[command(about = "A small dotfiles and configuration manager")]
#[command(version)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    #[command(about = "Create Lattice config and storage directories")]
    Init {
        #[arg(long)]
        force: bool,
    },
    #[command(about = "Print config paths and optional tool availability")]
    Doctor,
    #[command(about = "Validate global and service configuration")]
    Validate,
    #[command(about = "Manage service configuration entries")]
    Service {
        #[command(subcommand)]
        command: ServiceCommands,
    },
    #[command(about = "Add or remove include globs for a service")]
    Include {
        #[command(subcommand)]
        command: PatternCommands,
    },
    #[command(about = "Add or remove exclude globs for a service")]
    Exclude {
        #[command(subcommand)]
        command: PatternCommands,
    },
    #[command(about = "Manage restore permission rules")]
    Permission {
        #[command(subcommand)]
        command: PermissionCommands,
    },
    #[command(about = "Manage app catalog shortcuts")]
    App {
        #[command(subcommand)]
        command: AppCommands,
    },
    #[command(about = "Inspect service groups without mutating state")]
    Group {
        #[command(subcommand)]
        command: GroupCommands,
    },
    #[command(about = "Show active context labels for this machine")]
    Context {
        #[command(subcommand)]
        command: ContextCommands,
    },
    #[command(about = "Check new-machine readiness without mutating state")]
    Bootstrap {
        #[command(subcommand)]
        command: BootstrapCommands,
    },
    #[command(about = "Run git operations for a service repo")]
    Repo {
        #[command(subcommand)]
        command: RepoCommands,
    },
    #[command(about = "Manage secret references for a service")]
    Secret {
        #[command(subcommand)]
        command: SecretCommands,
    },
    #[command(about = "Add tracked paths to a service")]
    Track {
        service: String,
        #[arg(required = true)]
        paths: Vec<String>,
    },
    #[command(about = "Copy existing local paths into a service repo")]
    Adopt {
        #[arg(long)]
        allow_secret_looking_files: bool,
        #[arg(long)]
        allow_metadata_loss: bool,
        service: String,
        #[arg(required = true)]
        paths: Vec<String>,
    },
    #[command(about = "Compare local files with the service repo")]
    Diff {
        #[arg(long)]
        json: bool,
        #[arg(long, action = clap::ArgAction::Append)]
        only: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        service: String,
    },
    #[command(about = "Open the interactive text UI")]
    Tui {
        #[arg(long)]
        dry_run: bool,
    },
    #[command(about = "Summarize backup and restore risk before changing files")]
    Plan {
        #[arg(long)]
        json: bool,
        #[arg(long, action = clap::ArgAction::Append)]
        only: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        service: String,
    },
    #[command(about = "Show backup and restore status for a service")]
    Status {
        #[arg(long)]
        json: bool,
        #[arg(long, action = clap::ArgAction::Append)]
        only: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        service: String,
    },
    #[command(about = "Copy selected local files into a service repo")]
    Backup {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        allow_secret_looking_files: bool,
        #[arg(long)]
        allow_metadata_loss: bool,
        #[arg(long, action = clap::ArgAction::Append)]
        only: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        service: String,
    },
    #[command(about = "Restore selected files from a service repo")]
    Restore {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long, action = clap::ArgAction::Append)]
        only: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        service: String,
    },
    #[command(about = "Inspect and prune restore safety snapshots")]
    Snapshot {
        #[command(subcommand)]
        command: SnapshotCommands,
    },
    #[command(about = "Restore files from a safety snapshot")]
    Undo {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
        snapshot: String,
        service: Option<String>,
    },
    #[command(about = "Suggest local service candidates without mutating state")]
    Discover {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum SnapshotCommands {
    #[command(about = "List recorded restore safety snapshots")]
    List {
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Show files captured in one safety snapshot")]
    Show {
        #[arg(long)]
        json: bool,
        snapshot: String,
        service: Option<String>,
    },
    #[command(about = "Delete old safety snapshots after keeping recent entries")]
    Prune {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long, default_value_t = 20)]
        keep: usize,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ServiceCommands {
    #[command(about = "List configured services")]
    List,
    #[command(about = "Print one service config file")]
    Show { service: String },
    #[command(about = "Create or overwrite a service config file")]
    Add {
        service: String,
        #[arg(long)]
        root: String,
        #[arg(long)]
        repo: Option<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        include: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        #[arg(long)]
        template: bool,
        #[arg(long)]
        symlink: bool,
        #[arg(long)]
        os: Option<String>,
        #[arg(long)]
        hostname: Option<String>,
        #[arg(long = "context", action = clap::ArgAction::Append)]
        contexts: Vec<String>,
        #[arg(long)]
        force: bool,
    },
    #[command(about = "Remove a service config file")]
    Remove {
        #[arg(long)]
        yes: bool,
        service: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum PatternCommands {
    #[command(about = "Add glob patterns to the service")]
    Add {
        service: String,
        #[arg(required = true)]
        patterns: Vec<String>,
    },
    #[command(about = "Remove glob patterns from the service")]
    Remove {
        service: String,
        #[arg(required = true)]
        patterns: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum PermissionCommands {
    #[command(about = "Set a restore mode for one path")]
    Set {
        service: String,
        path: String,
        mode: String,
    },
    #[command(about = "Remove a restore mode for one path")]
    Remove { service: String, path: String },
}

#[derive(Debug, Subcommand)]
pub(crate) enum AppCommands {
    #[command(about = "List built-in app catalog entries")]
    List,
    #[command(about = "Show the suggested config for an app")]
    Show { app: String },
    #[command(about = "Create a service config from an app catalog entry")]
    Add {
        app: String,
        #[arg(long)]
        root: String,
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        template: bool,
        #[arg(long)]
        symlink: bool,
        #[arg(long)]
        os: Option<String>,
        #[arg(long)]
        hostname: Option<String>,
        #[arg(long = "context", action = clap::ArgAction::Append)]
        contexts: Vec<String>,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ContextCommands {
    #[command(about = "Show the current profile, labels, OS, and hostname")]
    Show {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum GroupCommands {
    #[command(about = "List configured service groups")]
    List {
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Show one service group")]
    Show {
        #[arg(long)]
        json: bool,
        group: String,
    },
    #[command(about = "Show grouped service status")]
    Status {
        #[arg(long)]
        json: bool,
        #[arg(long, action = clap::ArgAction::Append)]
        only: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        group: String,
    },
    #[command(about = "Summarize grouped service plans")]
    Plan {
        #[arg(long)]
        json: bool,
        #[arg(long, action = clap::ArgAction::Append)]
        only: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        group: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum BootstrapCommands {
    #[command(about = "Report readiness and recommended next actions")]
    Check {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum RepoCommands {
    #[command(about = "Show git status for a service repo")]
    Status { service: String },
    #[command(about = "Pull changes into a service repo")]
    Pull { service: String },
    #[command(about = "Commit service repo changes")]
    Commit {
        service: String,
        #[arg(short, long)]
        message: String,
    },
    #[command(about = "Push service repo changes")]
    Push { service: String },
}

#[derive(Debug, Subcommand)]
pub(crate) enum SecretCommands {
    #[command(about = "List secret references for a service")]
    List { service: String },
    #[command(about = "Add a secret reference to a service")]
    Add {
        service: String,
        name: String,
        #[arg(long)]
        backend: String,
        #[arg(long)]
        item: Option<String>,
        #[arg(long)]
        field: Option<String>,
        #[arg(long)]
        env: Option<String>,
        #[arg(long)]
        folder: Option<String>,
    },
    #[command(about = "Remove a secret reference from a service")]
    Remove { service: String, name: String },
    #[command(about = "Check whether configured secret backends are available")]
    Check { service: String },
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn top_level_command_inventory_stays_stable() {
        let commands: Vec<_> = Cli::command()
            .get_subcommands()
            .map(|command| command.get_name().to_string())
            .collect();

        assert_eq!(
            commands,
            vec![
                "init",
                "doctor",
                "validate",
                "service",
                "include",
                "exclude",
                "permission",
                "app",
                "group",
                "context",
                "bootstrap",
                "repo",
                "secret",
                "track",
                "adopt",
                "diff",
                "tui",
                "plan",
                "status",
                "backup",
                "restore",
                "snapshot",
                "undo",
                "discover",
            ]
        );
    }
}
