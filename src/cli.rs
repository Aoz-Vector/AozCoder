//! Command-line interface definition via clap derive macros.
//!
//! The top-level `Cli` struct accepts global flags (API URL, key, session ID)
//! that apply to all subcommands.  When no subcommand is given, `chat` is
//! implied per the `default_value_t` approach used in `main`.

use clap::{Parser, Subcommand, ValueEnum};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(
    name = "aozcoder",
    version = env!("CARGO_PKG_VERSION"),
    about = "Terminal UI client for the Vexcoder normalized inference API",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Base URL of the Vexcoder server.
    #[arg(
        short = 'u',
        long,
        env = "AOZCODER_API_URL",
        default_value = "http://localhost:8080"
    )]
    pub api_url: String,

    /// Bearer token for authenticated endpoints (RFC 6750 §2.1).
    #[arg(short = 'k', long, env = "AOZCODER_API_KEY")]
    pub api_key: Option<String>,

    /// Session identifier; generated as a UUIDv4 when omitted.
    #[arg(short = 's', long, env = "AOZCODER_SESSION_ID")]
    pub session_id: Option<String>,

    /// Enable debug-level tracing to stderr.
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start an interactive TUI session (default when no subcommand is given).
    Chat,

    /// Send a single prompt, print the response, and exit.
    Run {
        /// The prompt text.
        prompt: String,

        /// Response format written to stdout.
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Show session metadata and exit.
    Session,

    /// Print or edit the configuration file.
    Config {
        /// Open the config file in `$EDITOR`.
        #[arg(short, long)]
        edit: bool,
    },
}

/// Output format for the `run` subcommand.
#[derive(ValueEnum, Clone, Copy, Debug, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Markdown,
}

impl Cli {
    /// Returns the session ID, generating a UUIDv4 if none was supplied.
    pub fn session_id(&self) -> String {
        self.session_id
            .clone()
            .unwrap_or_else(|| format!("aoz-{}", Uuid::new_v4()))
    }
}
