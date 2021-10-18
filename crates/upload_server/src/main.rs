use anyhow::{Context, Error};
use log::*;
use rand::Rng;
use rouille::{Request, Response};
use std::{
    cmp::Ordering,
    env,
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, Write},
    path::Path,
    time::Duration,
};

const PLAYERS_DIRECTORY: &str = "crates/bomber_game/assets/players";
const API_KEYS_FILE: &str = "api_keys.txt";

const MAX_WASM_SIZE: usize = 10_000_000;
const WASM_FILE_PREFIX: &[u8] = b"\0asm";

const BAD_REQUEST: u16 = 400;
const UNAUTHORIZED: u16 = 401;
const METHOD_NOT_ALLOWED: u16 = 405;
const INTERNAL_SERVER_ERROR: u16 = 500;

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

/// Reads or creates file with API keys for players, returns a slice of valid keys.
fn init_api_keys(key_count: usize) -> Result<Vec<String>, Error> {
    let mut file =
        OpenOptions::new().read(true).write(true).create(true).open(API_KEYS_FILE).with_context(
            || format!("Opening {} for reading, writing and creating.", API_KEYS_FILE),
        )?;

    let keys: Result<Vec<_>, _> = BufReader::new(&file).lines().collect();
    let mut keys = keys?;

    match keys.len().cmp(&key_count) {
        Ordering::Greater => {
            warn!(
                "Read {} API keys from {} which is more than requested. Using first {} keys.",
                keys.len(),
                API_KEYS_FILE,
                key_count
            );
            keys.truncate(key_count);
        },
        Ordering::Equal => {
            info!("Read {} keys from {}. Using them all.", key_count, API_KEYS_FILE);
        },
        Ordering::Less => {
            info!(
                "Read {} keys from {}. Generating more to have {} and re-saving the file.",
                keys.len(),
                API_KEYS_FILE,
                key_count
            );
            let mut random_generator = rand::thread_rng();
            while keys.len() < key_count {
                let random: u32 = random_generator.gen();
                keys.push(format!("{:x}", random));
            }

            // Truncate the file and overwrite.
            file.set_len(0)?;
            // Set cursor at the beginning of the file, set_len() is documented not to do so.
            file.rewind()?;
            for line in keys.iter() {
                writeln!(&mut file, "{}", line)?;
            }
        },
    }

    Ok(keys)
}

fn handler(request: &Request, api_keys: &[String]) -> Response {
    if request.method() != "POST" {
        return Response::text("We only accept HTTP POST.\n").with_status_code(METHOD_NOT_ALLOWED);
    }

    let api_key = request.header("Api-Key").unwrap_or_default();
    if api_keys.iter().all(|allowed_key| allowed_key != api_key) {
        return Response::text("HTTP header Api-Key not present or not matching.\n")
            .with_status_code(UNAUTHORIZED);
    }

    if let Some(mut body) = request.data() {
        let mut data = Vec::new();
        if let Err(e) = body.read_to_end(&mut data) {
            return Response::text(format!("Failed to read input body: {}\n", e))
                .with_status_code(INTERNAL_SERVER_ERROR);
        }
        if data.len() > MAX_WASM_SIZE {
            return Response::text(format!("Maximum size of {} exceeded.\n", MAX_WASM_SIZE))
                .with_status_code(BAD_REQUEST);
        }
        if !data.starts_with(WASM_FILE_PREFIX) {
            return Response::text("Uploaded data not a WASM file.\n")
                .with_status_code(BAD_REQUEST);
        }
        match handle_upload(api_key, &data) {
            Ok(()) => Response::text("Your submission has been accepted.\n"),
            Err(e) => Response::text(format!("Error accepting your submission: {:#}\n", e))
                .with_status_code(INTERNAL_SERVER_ERROR),
        }
    } else {
        Response::text("Please submit request with body.\n").with_status_code(BAD_REQUEST)
    }
}

fn handle_upload(api_key: &str, data: &[u8]) -> Result<(), Error> {
    let path = Path::new(PLAYERS_DIRECTORY).join(format!("{}.wasm", api_key));

    let random: u32 = rand::thread_rng().gen();
    let temp_path = path.with_extension(format!("wasm.tmp{}", random));

    // Writing is not atomic, so write to temp file and then rename.
    fs::write(&temp_path, data).with_context(|| format!("writing {:?}", temp_path))?;
    fs::rename(&temp_path, &path)?;
    info!("{:?} saved.", path);
    Ok(())
}
