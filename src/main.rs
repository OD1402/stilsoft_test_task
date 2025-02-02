use anyhow::{anyhow, Context, Result};
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use sled::Db;

use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};

use tokio::time::Duration;

use axum::{
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use futures::StreamExt;
use sha2::{Digest, Sha256};
use std::io::BufRead;
use std::net::SocketAddr;

use chrono::{DateTime, Utc};

mod db;
use db::*;

mod engine;
use engine::*;

mod cli;
use cli::*;

mod server;
use server::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "config.toml")]
    config: PathBuf,

    #[arg(long)]
    max_requests_at_once: Option<usize>,
    #[arg(long)]
    fetch_timeout_secs: Option<u64>,

    #[command(subcommand)]
    pub cmd: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Cli {
        #[arg(short, long)]
        clear_db: bool,

        args: Vec<String>,
    },
    Server {
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
}

#[derive(Deserialize)]
pub struct Config {
    pub max_requests_at_once: usize,
    pub fetch_timeout_secs: u64,
    pub db_name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let Args {
        max_requests_at_once,
        fetch_timeout_secs,
        config,
        cmd,
    } = Args::parse();

    let env_file_name = PathBuf::from(".env");

    if env_file_name.exists() {
        dotenv::dotenv().map_err(|err| {
            anyhow!(
                "failed to open file '.env' in current dir {:?}: {err}",
                std::env::current_dir().unwrap_or("unknown".into())
            )
        })?;
    }

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let builder = config::Config::builder().add_source(config::File::from(config));
    let config: Config = builder.build()?.try_deserialize()?;

    match cmd {
        Some(Command::Server { port }) => {
            start_server(port, config).await;
        }
        Some(Command::Cli { args, clear_db }) => {
            run_cli(
                args,
                clear_db,
                max_requests_at_once,
                fetch_timeout_secs,
                config,
            )
            .await?;
        }
        None => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests;
