use anyhow::Result;
use clap::{Parser, Subcommand};
use semver;
use std::path::PathBuf;
use velopack::bundle::Manifest;
use velopack_bins::shared::dialogs;

#[derive(Parser)]
#[command(name = "windows_dialog_test", about = "A tool to test Velopack Windows dialogs")]
struct Cli {
    /// Force theme: "light" or "dark"
    #[arg(long, value_parser = ["light", "dark"])]
    theme: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show 'Restart Required' dialog
    Restart,
    /// Show 'Missing Dependencies' dialog (Setup version)
    Deps {
        /// Name of the missing dependency
        #[arg(short, long, default_value = "Test Dependency")]
        name: String,
    },
    /// Show 'Update Missing Dependencies' dialog
    UpdateDeps {
        /// Name of the missing dependency
        #[arg(short, long, default_value = "DotNet 8.0")]
        name: String,
        /// Current version
        #[arg(short, long, default_value = "1.0.0")]
        from: String,
        /// Target version
        #[arg(short, long, default_value = "1.1.0")]
        to: String,
    },
    /// Show 'Uninstall Complete with Errors' dialog
    UninstallError {
        /// Path to the log file (optional)
        #[arg(short, long)]
        log: Option<PathBuf>,
    },
    /// Show 'Processes Locking Folder' dialog
    Locked {
        /// Comma-separated list of process names
        #[arg(short, long, default_value = "Discord.exe, Chrome.exe")]
        procs: String,
    },
    /// Show 'Overwrite Repair' dialog
    Repair {
        /// Version to show in the dialog
        #[arg(short, long, default_value = "2.0.0")]
        version: String,
        /// Root path (optional)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// Show a progress dialog
    Progress {
        /// Content of the dialog
        #[arg(short, long, default_value = "Downloading update...")]
        content: String,
    },
    /// Show a splash screen
    Splash {
        /// App name
        #[arg(short, long, default_value = "Test App")]
        name: String,
    },
}

fn create_mock_manifest(version: &str) -> Manifest {
    let mut m = Manifest::default();
    m.title = "Test App".to_string();
    m.version = semver::Version::parse(version).unwrap();
    m.id = "test.app".to_string();
    m
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let _ = dialogs::set_theme_override_from_str(cli.theme.as_deref());
    let _ = velopack_bins::windows::splash::set_theme_override_from_str(cli.theme.as_deref());

    match cli.command {
        Commands::Restart => {
            let m = create_mock_manifest("1.0.0");
            dialogs::show_restart_required(&m);
        }
        Commands::Deps { name } => {
            let m = create_mock_manifest("1.0.0");
            let result = dialogs::show_setup_missing_dependencies_dialog(&m, &name);
            println!("Result: {}", result);
        }
        Commands::UpdateDeps { name, from, to } => {
            let m = create_mock_manifest(&to);
            let from_v = semver::Version::parse(&from)?;
            let to_v = semver::Version::parse(&to)?;
            let result = dialogs::show_update_missing_dependencies_dialog(&m, &name, &from_v, &to_v);
            println!("Result: {}", result);
        }
        Commands::UninstallError { log } => {
            dialogs::show_uninstall_complete_with_errors_dialog("Test App", log.as_ref());
        }
        Commands::Locked { procs } => {
            let result = dialogs::show_processes_locking_folder_dialog("Test App", "1.0.0", &procs);
            let s: &'static str = result.into();
            println!("Result: {}", s);
        }
        Commands::Repair { version, path } => {
            let m = create_mock_manifest(&version);
            let root = path.unwrap_or_else(|| std::env::current_dir().unwrap());
            let result = dialogs::show_overwrite_repair_dialog(&m, &root);
            println!("Result: {}", result);
        }
        Commands::Progress { content } => {
            let tx = velopack_bins::windows::splash::show_progress_dialog("Test App", &content);
            for i in (0..=100).step_by(10) {
                if tx.send(i as i16).is_err() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            let _ = tx.send(velopack_bins::windows::splash::MSG_CLOSE);
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
        Commands::Splash { name } => {
            let tx = velopack_bins::windows::splash::show_splash_dialog(name, None);
            for i in (0..=100).step_by(5) {
                if tx.send(i as i16).is_err() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            let _ = tx.send(velopack_bins::windows::splash::MSG_CLOSE);
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    }

    Ok(())
}
