use anyhow::{Context, Error};
use log::*;
use rand::Rng;
use std::{
    cmp::Ordering,
    fs::OpenOptions,
    io::{BufRead, BufReader, Seek, Write},
};

const API_KEYS_FILE: &str = "api_keys.txt";

/// Reads or creates file with API keys for players, returns a slice of valid keys.
pub fn init_api_keys(key_count: usize) -> Result<Vec<String>, Error> {
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
