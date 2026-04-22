use std::{
    error::Error,
    io::{self, Write},
    process::Command,
};

use clap::Parser;

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepoSpec {
    owner: String,
    repo: String,
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
    let files = git_stdout(["ls-files"])?;
    let mut stdout = io::BufWriter::new(io::stdout().lock());

    for path in files.lines().filter(|line| !line.is_empty()) {
        let url = build_raw_url(&cli.base_url, &repo, &branch, path);
        writeln!(stdout, "{path}\t{url}")?;
    }

    Ok(())
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
}
