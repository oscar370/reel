use std::path::{Path, PathBuf};

use gettextrs::gettext;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Path does not exist: {0}")]
    PathNotFound(PathBuf),
    #[error("Unsupported file: {0}")]
    UnsupportedFile(PathBuf),
    #[error("No supported files in folder: {0}")]
    NoSupportedFiles(PathBuf),
    #[error("Failed to load image: {0}")]
    LoadFailed(PathBuf),
}

pub fn generic_user_message() -> String {
    gettext("Something went wrong.")
}

impl AppError {
    pub fn user_message(&self) -> String {
        match self {
            Self::PathNotFound(path) => {
                // Translators: {name} is a file name. Do not translate {name}.
                gettext("“{name}” could not be found.").replace("{name}", &display_name(path))
            }
            Self::UnsupportedFile(path) => {
                // Translators: {name} is a file name. Do not translate {name}.
                gettext("“{name}” is not a supported file.").replace("{name}", &display_name(path))
            }
            Self::NoSupportedFiles(path) => {
                // Translators: {name} is a file name. Do not translate {name}.
                gettext("“{name}” does not contain any supported files.")
                    .replace("{name}", &display_name(path))
            }
            Self::LoadFailed(path) => {
                // Translators: {name} is a file name. Do not translate {name}.
                gettext("“{name}” could not be loaded.").replace("{name}", &display_name(path))
            }
        }
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| gettext("Unknown item"))
}
