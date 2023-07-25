/*!
This module contains platform-independent path normalization.
*/

// spell-checker:ignore () canonicalize canonicalization
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

use derive_builder::Builder;

// for WinOS, to specify a path which ends in a reserved name, start the path with ".\" or "./"
// * similar to the method used to exclude a path starting with a dash (eg, '-x') from being interpreted as an option
// for WinOS, trailing dots ('.') are removed unless the path is in UNC format? *or* keep for platform equivalence?

#[cfg(windows)]
const RESERVED_NAMES: [&'static str; 22] = [
    "AUX", "NUL", "PRN", "CON", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

// is_reserved_path(path: &str) -> bool
/// exact, case-insensitive comparison to reserved names
// #[cfg_attr(not(windows), allow(dead_code))]
pub fn is_reserved_path(path: &OsStr) -> bool {
    #[cfg(windows)]
    {
        // exact, case-insensitive comparison to reserved names (which are all uppercase, ASCII-character-only strings)
        // * to_string_lossy() is lossless when converting ASCII-character-only strings
        if RESERVED_NAMES.contains(&path.to_string_lossy().to_ascii_uppercase().as_str()) {
            return true;
        };
    }
    let _ = path; // suppress unused warning
    false
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum NormalizeMode {
    /// Normalize the path only if it (or parent) exists (ie, as `fs::canonicalize()`).
    Strict,
    /// Normalize the path only if it (or *any* parent) exists, normalizing lexically for later paths.
    /// * note: this will check for the existence of the nearest parent path, which may be expensive
    Hybrid,
    /// Normalize the path lexically, based solely on the path text, if the path (or parent) does not exist.
    #[default]
    Lexical,
}

#[derive(Builder, Clone, Debug, Default, PartialEq)]
pub struct NormalizeOptions {
    // #[builder(default = "NormalizeMode::Lexical")]
    mode: NormalizeMode,
}

pub fn normalize_path_with_options<P: AsRef<Path> + std::fmt::Debug>(
    path: P,
    options: &NormalizeOptions,
) -> std::io::Result<PathBuf> {
    let path = path.as_ref();
    if is_reserved_path(&path.as_os_str()) {
        return Ok(path.to_string_lossy().to_ascii_uppercase().into());
    };
    // avoid TOC/TOU race condition for path existence/fs::canonicalize() by checking result instead
    let result = fs::canonicalize(path);
    match result {
        Ok(pathbuf) => return Ok(dunce::simplified(&pathbuf).to_path_buf()),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound && options.mode == NormalizeMode::Strict {
                return Err(e);
            }
        }
    };
    let mut normalized_pathbuf = fs::canonicalize(".")?;
    for component in path.components() {
        match component {
            Component::ParentDir => {
                normalized_pathbuf.pop();
            }
            Component::CurDir => {}
            _ => {
                normalized_pathbuf.push(component);
            }
        }
    }
    let result = dunce::simplified(&normalized_pathbuf);
    Ok(result.to_path_buf())
}

pub fn normalize_path<P: AsRef<Path> + std::fmt::Debug>(path: P) -> std::io::Result<PathBuf> {
    normalize_path_with_options(path, &NormalizeOptions::default())
}
