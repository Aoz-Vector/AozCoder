use std::{
    error::Error,
    fs,
    io::{self, Write},
    path::PathBuf,
    process::Command,
};

use clap::{Parser, ValueEnum};

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Tsv,
    Markdown,
}

#[derive(Parser, Debug)]
#[command(name = "raw-links")]
struct Cli {
    #[arg(long)]
    owner: Option<String>,

    #[arg(long)]
    repo: Option<String>,

    #[arg(long)]
    branch: Option<String>,

    #[arg(long, default_value = "origin")]
    remote: String,

    #[arg(long, default_value = "https://raw.githubusercontent.com")]
    base_url: String,

    #[arg(long, value_enum, default_value_t = OutputFormat::Tsv)]
    format: OutputFormat,

    #[arg(long)]
    output: Option<PathBuf>,

    #[arg(long, default_value_t = false)]
    check: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepoSpec {
    owner: String,
    repo: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LinkEntry {
    path: String,
    url: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let repo = match (cli.owner, cli.repo) {
        (Some(owner), Some(repo)) => RepoSpec { owner, repo },
        (Some(_), None) | (None, Some(_)) => {
            return Err(io::Error::other("--owner and --repo must be supplied together").into());
        }
        (None, None) => parse_github_remote(&git_stdout(["remote", "get-url", &cli.remote])?)
            .ok_or_else(|| io::Error::other("failed to infer owner/repo from git remote"))?,
    };

    let branch = cli.branch.unwrap_or_else(default_branch);
    let entries = collect_entries(&cli.base_url, &repo, &branch)?;
    let rendered = match cli.format {
        OutputFormat::Tsv => render_tsv(&entries),
        OutputFormat::Markdown => render_markdown(&entries, &repo, &branch, &cli.base_url),
    };

    match (cli.output.as_ref(), cli.check) {
        (Some(path), true) => check_output(path, &rendered)?,
        (Some(path), false) => write_output(path, &rendered)?,
        (None, true) => {
            return Err(io::Error::other("--check requires --output").into());
        }
        (None, false) => {
            let mut stdout = io::BufWriter::new(io::stdout().lock());
            stdout.write_all(rendered.as_bytes())?;
        }
    }

    Ok(())
}

fn collect_entries(
    base_url: &str,
    repo: &RepoSpec,
    branch: &str,
) -> Result<Vec<LinkEntry>, io::Error> {
    let files = git_stdout(["ls-files"])?;

    Ok(files
        .lines()
        .filter(|line| !line.is_empty())
        .map(|path| LinkEntry {
            path: path.to_string(),
            url: build_raw_url(base_url, repo, branch, path),
        })
        .collect())
}

fn build_raw_url(base_url: &str, repo: &RepoSpec, branch: &str, path: &str) -> String {
    format!(
        "{}/{}/{}/{}/{}",
        base_url.trim_end_matches('/'),
        encode_segment(&repo.owner),
        encode_segment(&repo.repo),
        encode_segment(branch),
        encode_path(path)
    )
}

fn render_tsv(entries: &[LinkEntry]) -> String {
    let mut rendered = String::new();
    for entry in entries {
        rendered.push_str(&entry.path);
        rendered.push('\t');
        rendered.push_str(&entry.url);
        rendered.push('\n');
    }
    rendered
}

fn render_markdown(entries: &[LinkEntry], repo: &RepoSpec, branch: &str, base_url: &str) -> String {
    let mut rendered = String::new();
    rendered.push_str("# Raw URL Sitemap\n\n");
    rendered.push_str("This file is generated from `git ls-files`.\n\n");
    rendered.push_str("Repository: `");
    rendered.push_str(&repo.owner);
    rendered.push('/');
    rendered.push_str(&repo.repo);
    rendered.push_str("`\n\n");
    rendered.push_str("Branch: `");
    rendered.push_str(branch);
    rendered.push_str("`\n\n");
    rendered.push_str("Base URL: `");
    rendered.push_str(&build_raw_url(base_url, repo, branch, ""));
    rendered.push_str("`\n\n");

    for entry in entries {
        rendered.push_str("- [");
        rendered.push_str(&entry.path);
        rendered.push_str("](");
        rendered.push_str(&entry.url);
        rendered.push_str(")\n");
    }

    rendered
}

fn write_output(path: &PathBuf, rendered: &str) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, rendered)
}

fn check_output(path: &PathBuf, rendered: &str) -> Result<(), io::Error> {
    let existing = fs::read_to_string(path)?;
    if existing == rendered {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{} is stale; regenerate the raw URL sitemap",
            path.display()
        )))
    }
}

fn default_branch() -> String {
    match git_stdout(["rev-parse", "--abbrev-ref", "HEAD"]) {
        Ok(branch) => {
            let branch = branch.trim();
            if branch.is_empty() || branch == "HEAD" {
                "main".to_string()
            } else {
                branch.to_string()
            }
        }
        Err(_) => "main".to_string(),
    }
}

fn git_stdout<const N: usize>(args: [&str; N]) -> Result<String, io::Error> {
    let output = Command::new("git").args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

fn parse_github_remote(remote: &str) -> Option<RepoSpec> {
    let trimmed = remote.trim().trim_end_matches('/');
    let path = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
        .or_else(|| trimmed.strip_prefix("git@github.com:"))
        .or_else(|| trimmed.strip_prefix("ssh://git@github.com/"))?
        .trim_end_matches(".git");

    let mut parts = path.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;

    if parts.next().is_some() || owner.is_empty() || repo.is_empty() {
        return None;
    }

    Some(RepoSpec {
        owner: owner.to_string(),
        repo: repo.to_string(),
    })
}

fn encode_segment(segment: &str) -> String {
    percent_encode(segment.as_bytes(), false)
}

fn encode_path(path: &str) -> String {
    percent_encode(path.as_bytes(), true)
}

fn percent_encode(bytes: &[u8], keep_slash: bool) -> String {
    let mut encoded = String::with_capacity(bytes.len());
    for &byte in bytes {
        let ch = byte as char;
        let safe = ch.is_ascii_alphanumeric()
            || matches!(ch, '-' | '_' | '.' | '~')
            || (keep_slash && ch == '/');
        if safe {
            encoded.push(ch);
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_https_remote() {
        assert_eq!(
            parse_github_remote("https://github.com/Aoz-Vector/AozCoder.git"),
            Some(RepoSpec {
                owner: "Aoz-Vector".to_string(),
                repo: "AozCoder".to_string(),
            })
        );
    }

    #[test]
    fn parse_ssh_remote() {
        assert_eq!(
            parse_github_remote("git@github.com:Aoz-Vector/AozCoder.git"),
            Some(RepoSpec {
                owner: "Aoz-Vector".to_string(),
                repo: "AozCoder".to_string(),
            })
        );
    }

    #[test]
    fn encode_path_preserves_separator() {
        assert_eq!(
            encode_path("docs/spec file.md"),
            "docs/spec%20file.md".to_string()
        );
    }

    #[test]
    fn build_raw_url_encodes_branch_segment() {
        let repo = RepoSpec {
            owner: "Aoz-Vector".to_string(),
            repo: "AozCoder".to_string(),
        };

        assert_eq!(
            build_raw_url(
                "https://raw.githubusercontent.com",
                &repo,
                "work/topic",
                "src/main.rs"
            ),
            "https://raw.githubusercontent.com/Aoz-Vector/AozCoder/work%2Ftopic/src/main.rs"
                .to_string()
        );
    }

    #[test]
    fn render_markdown_includes_heading_and_links() {
        let repo = RepoSpec {
            owner: "Aoz-Vector".to_string(),
            repo: "AozCoder".to_string(),
        };
        let entries = vec![LinkEntry {
            path: "src/main.rs".to_string(),
            url: "https://raw.githubusercontent.com/Aoz-Vector/AozCoder/main/src/main.rs"
                .to_string(),
        }];

        let rendered =
            render_markdown(&entries, &repo, "main", "https://raw.githubusercontent.com");

        assert!(rendered.starts_with("# Raw URL Sitemap\n"));
        assert!(rendered.contains(
            "- [src/main.rs](https://raw.githubusercontent.com/Aoz-Vector/AozCoder/main/src/main.rs)"
        ));
    }
}
