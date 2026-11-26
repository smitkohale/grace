//! `grace`: Grace's command line, for power users and automation (spec §7.2).
//! Everyday use happens in the desktop app; nothing here is needed to use Grace.

mod assets;
mod output;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use grace_core::Redactor;
use grace_core::api::{self, ApiOptions};
use grace_core::config::Config;
use grace_core::discover::{self, DiscoveryEnv};
use grace_core::scan;
use grace_core::store::{SessionQuery, Store};
use std::process::{Command as Process, ExitCode};

#[derive(Parser)]
#[command(
    name = "grace",
    version,
    about = "Grace keeps your AIs on the same page. Everything stays on this computer."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show which AI tools are installed and where their sessions are. Read-only.
    Doctor {
        /// Print JSON instead of a table.
        #[arg(long)]
        json: bool,
    },
    /// List your AI sessions, most recent first.
    Sessions {
        /// Only sessions for this project folder.
        #[arg(long)]
        project: Option<String>,
        /// Only sessions whose title, prompts or project match this text.
        #[arg(long)]
        search: Option<String>,
        /// How many sessions to show.
        #[arg(long, default_value_t = 20)]
        limit: u32,
        /// Print JSON instead of a table.
        #[arg(long)]
        json: bool,
        /// Don't read new session data first.
        #[arg(long)]
        no_scan: bool,
    },
    /// Run Grace's web UI on this computer (for headless or remote machines).
    Serve {
        /// Port on 127.0.0.1; 0 picks a free one.
        #[arg(long, default_value_t = 0)]
        port: u16,
        /// Open it in your browser.
        #[arg(long)]
        open: bool,
        /// Use this access token instead of a random one.
        #[arg(long)]
        token: Option<String>,
        /// Also accept requests from this web origin (for UI development).
        #[arg(long = "allow-origin")]
        allow_origin: Vec<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Doctor { json } => doctor(json),
        Command::Sessions {
            project,
            search,
            limit,
            json,
            no_scan,
        } => sessions(project, search, limit, json, no_scan),
        Command::Serve {
            port,
            open,
            token,
            allow_origin,
        } => serve(port, open, token, allow_origin),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Grace couldn't finish: {err:#}");
            eprintln!(
                "Your data is safe: Grace only reads your sessions. Try again, or run `grace doctor` to check your setup."
            );
            ExitCode::FAILURE
        }
    }
}

fn doctor(json: bool) -> Result<()> {
    let report = discover::doctor(&DiscoveryEnv::from_system());
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", output::doctor(&report));
    }
    Ok(())
}

fn sessions(
    project: Option<String>,
    search: Option<String>,
    limit: u32,
    json: bool,
    no_scan: bool,
) -> Result<()> {
    let env = DiscoveryEnv::from_system();
    let config = Config::load(&env.grace_home)?;
    let mut store = Store::open(&env.grace_home.join("grace.db"))?;
    if !no_scan {
        let adapters = scan::adapters_for(&config, &env);
        let redactor = Redactor::new(config.redact_emails);
        let report = scan::scan(&mut store, &adapters, &redactor, &mut |_| {})?;
        for error in &report.errors {
            eprintln!("Couldn't read a session: {error}");
        }
    }
    let rows = store.list_sessions(&SessionQuery {
        q: search,
        project,
        limit,
    })?;
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        print!("{}", output::sessions(&rows, grace_core::timefmt::now_ms()));
    }
    Ok(())
}

fn serve(port: u16, open: bool, token: Option<String>, allow_origin: Vec<String>) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
            .await
            .with_context(|| format!("couldn't use port {port} on 127.0.0.1"))?;
        let address = listener.local_addr()?;
        let token = token.unwrap_or_else(api::random_token);
        let env = DiscoveryEnv::from_system();
        let opts = ApiOptions {
            grace_home: env.grace_home.clone(),
            token: token.clone(),
            allowed_origins: allow_origin,
            discovery: env,
            ui: Some(assets::ui()),
        };
        let url = format!("http://{address}/#token={token}");
        println!("Grace is running at {url}");
        println!("Keep this window open while you use Grace. Press Ctrl+C to stop.");
        if open {
            open_browser(&url);
        }
        let shutdown = async {
            let _ = tokio::signal::ctrl_c().await;
        };
        api::run(listener, api::router(opts), shutdown).await?;
        Ok(())
    })
}

fn open_browser(url: &str) {
    let opened = if cfg!(windows) {
        Process::new("cmd").args(["/C", "start", "", url]).spawn()
    } else if cfg!(target_os = "macos") {
        Process::new("open").arg(url).spawn()
    } else {
        Process::new("xdg-open").arg(url).spawn()
    };
    if opened.is_err() {
        println!("Couldn't open a browser. Copy the address above into one.");
    }
}
