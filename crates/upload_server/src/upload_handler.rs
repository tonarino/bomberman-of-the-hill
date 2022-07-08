use anyhow::{anyhow, bail, Context, Error};
use log::*;
use rand::Rng;
use rouille::{Request, Response};
use std::{
    ffi::OsStr,
    fs::{self, create_dir_all},
    io::Read,
    path::{Path, PathBuf},
};

const ROUNDS_FOLDER: &str = "rounds";
/// Max number of rounds the upload server will attempt to create.
const MAX_ROUNDS: usize = 10_000;
/// Maximum number of players in a round before upload server starts putting them into next one.
const MAX_PLAYERS_PER_ROUND: usize = 8;
/// Name of the file that the game engine uses to mark a finished round.
const FINISHED_ROUND_MARKER_FILENAME: &str = "round-finished.marker";

const MAX_WASM_SIZE: usize = 10_000_000;
const WASM_FILE_PREFIX: &[u8] = b"\0asm";

const BAD_REQUEST: u16 = 400;
const UNAUTHORIZED: u16 = 401;
const METHOD_NOT_ALLOWED: u16 = 405;
const INTERNAL_SERVER_ERROR: u16 = 500;

pub fn handler(request: &Request, api_keys: &[String]) -> Response {
    if request.method() != "POST" {
        return text_response("We only accept HTTP POST.\n").with_status_code(METHOD_NOT_ALLOWED);
    }

    let api_key = match request.header("Api-Key") {
        Some(api_key) => {
            if api_keys.iter().all(|allowed_key| allowed_key != api_key) {
                return text_response(format!("HTTP header Api-Key \"{}\" not valid.\n", api_key))
                    .with_status_code(UNAUTHORIZED);
            }
            api_key
        },
        None => {
            return text_response("HTTP header Api-Key not present, please include it.")
                .with_status_code(UNAUTHORIZED)
        },
    };

    if let Some(mut body) = request.data() {
        let mut data = Vec::new();
        if let Err(e) = body.read_to_end(&mut data) {
            return text_response(format!("Failed to read input body: {}\n", e))
                .with_status_code(INTERNAL_SERVER_ERROR);
        }
        if data.len() > MAX_WASM_SIZE {
            return text_response(format!("Maximum size of {} exceeded.\n", MAX_WASM_SIZE))
                .with_status_code(BAD_REQUEST);
        }
        if !data.starts_with(WASM_FILE_PREFIX) {
            return text_response("Uploaded data not a WASM file.\n").with_status_code(BAD_REQUEST);
        }
        match handle_upload(api_key, &data) {
            Ok(round_number) => text_response(format!(
                "Your submission has been accepted to round {round_number}.\n"
            )),
            Err(e) => text_response(format!("Error accepting your submission: {:#}\n", e))
                .with_status_code(INTERNAL_SERVER_ERROR),
        }
    } else {
        text_response("Please submit request with body.\n").with_status_code(BAD_REQUEST)
    }
}

fn handle_upload(api_key: &str, data: &[u8]) -> Result<usize, Error> {
    let filename = format!("{}.wasm", api_key);
    let (round_number, path) = get_upload_round_and_path_for(&filename)?;

    let random: u32 = rand::thread_rng().gen();
    let temp_path = path.with_extension(format!("wasm.tmp{}", random));

    // Writing is not atomic, so write to temp file and then rename.
    fs::write(&temp_path, data).with_context(|| format!("writing {:?}", temp_path))?;
    fs::rename(&temp_path, &path)?;
    info!("{:?} saved.", path);
    Ok(round_number)
}

/// Return a path to upload `filename` player to, creating folders as necessary.
fn get_upload_round_and_path_for(filename: &str) -> Result<(usize, PathBuf), Error> {
    let rounds_path = Path::new(ROUNDS_FOLDER);
    if !rounds_path.is_dir() {
        bail!("{:?} must be a directory.", rounds_path);
    }

    for round in 1..MAX_ROUNDS {
        let round_path = rounds_path.join(round.to_string());

        // Skip finished rounds.
        if round_path.join(FINISHED_ROUND_MARKER_FILENAME).exists() {
            continue;
        }

        let player_in_round_path = round_path.join(filename);

        // If this player is already in some non-past round, overwrite it.
        if player_in_round_path.exists() {
            return Ok((round, player_in_round_path));
        }

        // The round folder may not exist, ensure it does.
        create_dir_all(&round_path)?;

        // Skip full rounds.
        if count_players_in_dir(&round_path)? >= MAX_PLAYERS_PER_ROUND {
            continue;
        }

        return Ok((round, player_in_round_path));
    }

    Err(anyhow!("Couldn't find a round to add player to."))
}

fn count_players_in_dir(path: &Path) -> Result<usize, Error> {
    let wasm_extension = OsStr::new("wasm");

    let mut count = 0;
    for file in path.read_dir().context(format!("reading {path:?}"))? {
        let path = file?.path();
        if path.is_file() && path.extension() == Some(wasm_extension) {
            count += 1;
        }
    }
    Ok(count)
}

/// Create a text response and log it. Work-around for the fact that response body can be read only
/// once from [rouille::Response]. Use instead of text_response(...).
fn text_response(text: impl Into<String>) -> Response {
    let text: String = text.into();
    debug!("Responding with: {}", text.trim());
    Response::text(text)
}
