use anyhow::{Context, Error};
use log::*;
use rand::Rng;
use rouille::{Request, Response};
use std::{fs, io::Read, path::Path};

const PLAYERS_DIRECTORY: &str = "crates/bomber_game/assets/players";

const MAX_WASM_SIZE: usize = 10_000_000;
const WASM_FILE_PREFIX: &[u8] = b"\0asm";

const BAD_REQUEST: u16 = 400;
const UNAUTHORIZED: u16 = 401;
const METHOD_NOT_ALLOWED: u16 = 405;
const INTERNAL_SERVER_ERROR: u16 = 500;

pub fn handler(request: &Request, api_keys: &[String]) -> Response {
    if request.method() != "POST" {
        return Response::text("We only accept HTTP POST.\n").with_status_code(METHOD_NOT_ALLOWED);
    }

    let api_key = match request.header("Api-Key") {
        Some(api_key) => {
            if api_keys.iter().all(|allowed_key| allowed_key != api_key) {
                return Response::text(format!("HTTP header Api-Key \"{}\" not valid.\n", api_key))
                    .with_status_code(UNAUTHORIZED);
            }
            api_key
        },
        None => {
            return Response::text("HTTP header Api-Key not present, please include it.")
                .with_status_code(UNAUTHORIZED)
        },
    };

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
