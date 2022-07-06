use crate::{api_keys::init_api_keys, upload_handler::handler};
use anyhow::{Context, Error};
use log::*;
use rouille::{Request, Response};
use std::{env, time::Duration};

mod api_keys;
mod upload_handler;

fn main() -> Result<(), Error> {
    let dotenv_result = dotenv::dotenv();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    match dotenv_result {
        Ok(path) => info!("Loaded env variables from {:?}.", path),
        Err(e) => info!(
            "Loading .env file: {}. Copy .env.example to .env to conveniently set env variables.",
            e
        ),
    }

    let key_count = env::var("API_KEY_COUNT")
        .unwrap_or_else(|_| "20".to_owned())
        .parse()
        .context("parsing ${API_KEY_COUNT} as usize")?;
    let api_keys = init_api_keys(key_count)?;

    let bind_addr = env::var("UPLOAD_SERVER_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8765".to_owned());

    let log_ok = |req: &Request, resp: &Response, elapsed: Duration| {
        info!(
            "{} {} {} {:?} {}",
            req.method(),
            req.raw_url(),
            req.remote_addr(),
            elapsed,
            resp.status_code
        );
    };
    let log_err = |req: &Request, elapsed: Duration| {
        error!(
            "Handler panicked: {} {} {} {:?}",
            req.method(),
            req.raw_url(),
            req.remote_addr(),
            elapsed
        );
    };

    rouille::start_server(bind_addr, move |req| {
        rouille::log_custom(req, log_ok, log_err, || handler(req, &api_keys))
    });
}
