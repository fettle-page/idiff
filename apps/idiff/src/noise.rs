//! The noise list a command uses: the built-in one plus the user's own rules from idiff's home.

use idiff_core::copy;
use idiff_core::noise::NoiseList;
use std::path::Path;

/// The user's own rules, which `idiff calibrate --save` adds to. Same format as the built-in list.
pub const LOCAL_FILE: &str = "noise.local.toml";

/// The built-in rules merged with `{home}\noise.local.toml`. A missing file means no local
/// rules. A file that can't be read or has a mistake isn't used at all, and the returned
/// warning says why, for the command to print on stderr.
pub fn load(home: &Path) -> (NoiseList, Option<String>) {
    let path = home.join(LOCAL_FILE);
    let local = match std::fs::read_to_string(&path) {
        Ok(text) => NoiseList::parse(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (NoiseList::builtin(), None),
        Err(e) => Err(e.to_string()),
    };
    match local {
        Ok(local) => (NoiseList::builtin().merged(local), None),
        Err(error) => (
            NoiseList::builtin(),
            Some(format!("{} ({}: {error})", copy::NOISE_LOCAL_BAD, path.display())),
        ),
    }
}
