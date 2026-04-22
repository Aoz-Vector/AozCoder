use anyhow::Result;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use aozcoder::cli::{Cli, Commands};
use aozcoder::client::SseClient;
use aozcoder::config::Config;
use aozcoder::print::{BatchOutput, OutputFormat};
use aozcoder::tui::EventLoop;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose {
        EnvFilter::new("aozcoder=debug,info")
    } else {
        EnvFilter::from_default_env()
            .add_directive("aozcoder=info".parse()?)
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    let config = Config::load().unwrap_or_else(|e| {
        tracing::warn!(error = %e, "config load failed; using defaults");
        Config {
            api_url: cli.api_url.clone(),
            api_key: cli.api_key.clone(),
            ui: Default::default(),
            model: Default::default(),
        }
    });

    let api_url = cli.api_url.clone();
    let api_key = cli.api_key.clone().or(config.api_key);
    let session_id = cli.session_id();

    let sse_client = SseClient::new(api_url, api_key, session_id.clone());

    match cli.command {
        None | Some(Commands::Chat) => {
            run_tui(sse_client, session_id).await?;
        }
        Some(Commands::Run { prompt, format }) => {
            run_batch(sse_client, session_id, prompt, format).await?;
        }
        Some(Commands::Session) => {
            println!("session_id: {session_id}");
            println!("api_url:    {}", config.api_url);
        }
        Some(Commands::Config { edit }) => {
            handle_config(edit)?;
        }
    }

    Ok(())
}

async fn run_tui(sse_client: SseClient, session_id: String) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    let event_loop = EventLoop::new(terminal, sse_client, session_id);
    let result = event_loop.run().await;

    disable_raw_mode()?;
    execute!(std::io::stdout(), LeaveAlternateScreen)?;

    result
}

async fn run_batch(
    sse_client: SseClient,
    session_id: String,
    prompt: String,
    format: aozcoder::cli::OutputFormat,
) -> Result<()> {
    let fmt = match format {
        aozcoder::cli::OutputFormat::Text => OutputFormat::Text,
        aozcoder::cli::OutputFormat::Json => OutputFormat::Json,
        aozcoder::cli::OutputFormat::Markdown => OutputFormat::Markdown,
    };

    let body = serde_json::json!({
        "task": prompt,
        "stream": true,
        "session_id": session_id,
    });

    let mut stream = sse_client.connect_stream("v1/run", body).await?;
    let mut output = BatchOutput::new(fmt);

    while let Some(result) = stream.next().await {
        match result {
            Ok(envelope) => {
                if output.ingest(envelope) {
                    break;
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("stream error: {e}"));
            }
        }
    }

    output.flush()?;
    Ok(())
}

fn handle_config(edit: bool) -> Result<()> {
    let path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("aozcoder")
        .join("config.toml");

    if edit {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        std::process::Command::new(editor).arg(&path).status()?;
    } else {
        println!("config file: {}", path.display());
        println!("set EDITOR and run: aozcoder config --edit");
    }

    Ok(())
}
