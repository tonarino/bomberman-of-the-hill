use anyhow::{Context, Error};
use log::*;
use rand::Rng;
use rouille::{Request, Response};
use std::{env, fs, io::Read, path::Path, time::Duration};

const PLAYERS_DIRECTORY: &str = "crates/bomber_game/assets/players";

// TODO(Matej): load this from file.
const API_KEYS: &[&str] = &["abcdef", "123456"];

const MAX_WASM_SIZE: usize = 10_000_000;
const WASM_FILE_PREFIX: &[u8] = b"\0asm";

fn main() {
    // TODO(Matej): load env from dotenv.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

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
        rouille::log_custom(req, log_ok, log_err, || handler(req))
    });
}

fn handler(request: &Request) -> Response {
    if request.method() != "POST" {
        return Response::text("We only accept HTTP POST.\n").with_status_code(405);
    }

    let api_key = request.header("Api-Key").unwrap_or_default();
    if !API_KEYS.contains(&api_key) {
        return Response::text("HTTP header Api-Key not present or not matching.\n")
            .with_status_code(401);
    }

    if let Some(mut body) = request.data() {
        let mut data = Vec::new();
        if let Err(e) = body.read_to_end(&mut data) {
            return Response::text(format!("Failed to read input body: {}\n", e))
                .with_status_code(500);
        }
        if data.len() > MAX_WASM_SIZE {
            return Response::text(format!("Maximum size of {} exceeded.\n", MAX_WASM_SIZE))
                .with_status_code(400);
        }
        if !data.starts_with(WASM_FILE_PREFIX) {
            return Response::text("Uploaded data not a WASM file.\n").with_status_code(400);
        }
        match handle_upload(api_key, &data) {
            Ok(()) => Response::text("Your submission has been accepted.\n"),
            Err(e) => Response::text(format!("Error accepting your submission: {:#}\n", e))
                .with_status_code(500),
        }
    } else {
        Response::text("Please submit request with body.\n").with_status_code(400)
    }
}

fn handle_upload(api_key: &str, data: &[u8]) -> Result<(), Error> {
    let path = Path::new(PLAYERS_DIRECTORY).join(format!("{}.wasm", api_key));

    let random: u32 = rand::thread_rng().gen();
    let temp_path = path.with_extension(format!("wasm.tmp{}", random));

    // Writing is not atomic, so write to temp file and them rename.
    fs::write(&temp_path, data).with_context(|| format!("writing {:?}", temp_path))?;
    fs::rename(&temp_path, &path)?;
    info!("{:?} saved.", path);
    Ok(())
}
