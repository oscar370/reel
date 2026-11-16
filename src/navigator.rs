use std::cmp::Ordering;
use std::fs;
use std::iter::Peekable;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::errors::AppError;

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff", "ico", "svg", "avif", "heic",
    "heif", "jxl",
];
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "m4v", "mkv", "webm", "mov", "avi", "ogv"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Video,
}

fn normalize_extension(ext: &str) -> String {
    ext.trim_start_matches('.').to_ascii_lowercase()
}

fn is_media(ext: &str) -> Option<MediaKind> {
    let ext = normalize_extension(ext);

    if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        Some(MediaKind::Image)
    } else if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        Some(MediaKind::Video)
    } else {
        None
    }
}

pub fn media_kind(path: &Path) -> Option<MediaKind> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(is_media)
}

fn is_supported_file(path: &Path) -> bool {
    path.is_file() && media_kind(path).is_some()
}

fn parent_folder(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn take_ascii_digits<I: Iterator<Item = char>>(chars: &mut Peekable<I>) -> String {
    let mut digits = String::new();

    while let Some(digit) = chars.next_if(|c| c.is_ascii_digit()) {
        digits.push(digit);
    }

    digits
}

fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a_char), Some(b_char)) => {
                if a_char.is_ascii_digit() && b_char.is_ascii_digit() {
                    let a_number = take_ascii_digits(&mut a_chars);
                    let b_number = take_ascii_digits(&mut b_chars);
                    let a_digits = a_number.trim_start_matches('0');
                    let b_digits = b_number.trim_start_matches('0');

                    let ordering = a_digits
                        .len()
                        .cmp(&b_digits.len())
                        .then_with(|| a_digits.cmp(b_digits));

                    if ordering != Ordering::Equal {
                        return ordering;
                    }
                } else {
                    let a_char = a_chars.next().unwrap();
                    let b_char = b_chars.next().unwrap();

                    if a_char != b_char {
                        return a_char.cmp(&b_char);
                    }
                }
            }
        }
    }
}

fn file_sort_key(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn sort_files(files: &mut [PathBuf]) {
    files.sort_by(|a, b| {
        let a_key = file_sort_key(a);
        let b_key = file_sort_key(b);

        natural_cmp(&a_key, &b_key).then_with(|| a_key.cmp(&b_key))
    });
}

fn collect_supported_files(folder: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    let entries = fs::read_dir(folder)
        .with_context(|| format!("Failed to read folder: {}", folder.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("Failed to read an entry in: {}", folder.display()))?;
        let entry_path = entry.path();

        if is_supported_file(&entry_path) {
            files.push(entry_path);
        }
    }

    Ok(files)
}

#[derive(Debug)]
pub struct FolderNavigator {
    files: Vec<PathBuf>,
    current_index: usize,
}

impl FolderNavigator {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(AppError::PathNotFound(path.to_path_buf()).into());
        }

        if path.is_dir() {
            return Ok(Self {
                files: Self::files_in_folder(path)?,
                current_index: 0,
            });
        }

        let mut files = collect_supported_files(parent_folder(path))?;

        if !files
            .iter()
            .any(|file| file.file_name() == path.file_name())
        {
            files.push(path.to_path_buf());
        }

        sort_files(&mut files);

        let current_index = path
            .file_name()
            .and_then(|name| files.iter().position(|file| file.file_name() == Some(name)))
            .unwrap_or(0);

        Ok(Self {
            files,
            current_index,
        })
    }

    pub fn next(&mut self) -> PathBuf {
        self.current_index = (self.current_index + 1) % self.files.len();
        self.current_path()
    }

    pub fn previous(&mut self) -> PathBuf {
        self.current_index = if self.current_index == 0 {
            self.files.len() - 1
        } else {
            self.current_index - 1
        };
        self.current_path()
    }

    fn files_in_folder(folder: &Path) -> Result<Vec<PathBuf>> {
        let mut files = collect_supported_files(folder)?;

        if files.is_empty() {
            return Err(AppError::NoSupportedFiles(folder.to_path_buf()).into());
        }

        sort_files(&mut files);
        Ok(files)
    }

    pub fn current_path(&self) -> PathBuf {
        self.files[self.current_index].clone()
    }

    pub fn current_kind(&self) -> Option<MediaKind> {
        media_kind(&self.current_path())
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn has_multiple_files(&self) -> bool {
        self.files.len() > 1
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};

    use tempfile::tempdir;

    use super::*;

    fn touch(path: &Path) {
        File::create(path).unwrap();
    }

    #[test]
    fn folder_keeps_only_supported_files_sorted() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join("b.PNG"));
        touch(&dir.path().join("a.mkv"));
        touch(&dir.path().join("notes.txt"));
        fs::create_dir(dir.path().join("c.jpg")).unwrap();

        let navigator = FolderNavigator::new(dir.path()).unwrap();

        assert_eq!(
            navigator.files,
            vec![dir.path().join("a.mkv"), dir.path().join("b.PNG")]
        );
        assert_eq!(navigator.current_index, 0);
        assert!(!navigator.is_empty());
        assert!(navigator.has_multiple_files());
    }

    #[test]
    fn folder_uses_natural_case_insensitive_sort() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join("img1.jpg"));
        touch(&dir.path().join("img2.jpg"));
        touch(&dir.path().join("IMG3.jpg"));
        touch(&dir.path().join("img10.jpg"));

        let navigator = FolderNavigator::new(dir.path()).unwrap();

        assert_eq!(
            navigator.files,
            vec![
                dir.path().join("img1.jpg"),
                dir.path().join("img2.jpg"),
                dir.path().join("IMG3.jpg"),
                dir.path().join("img10.jpg"),
            ]
        );
    }

    #[test]
    fn natural_cmp_compares_numbers_and_following_characters() {
        use std::cmp::Ordering;

        assert_eq!(natural_cmp("file1-z", "file1a"), Ordering::Less);
        assert_eq!(natural_cmp("a1b", "a1c"), Ordering::Less);
        assert_eq!(natural_cmp("img2", "img10"), Ordering::Less);
        assert_eq!(natural_cmp("img2.jpg", "img10.jpg"), Ordering::Less);
    }

    #[test]
    fn media_kind_normalizes_extensions() {
        assert_eq!(media_kind(Path::new("a.JPG")), Some(MediaKind::Image));
        assert_eq!(media_kind(Path::new("a.png")), Some(MediaKind::Image));
        assert_eq!(media_kind(Path::new("a.webp")), Some(MediaKind::Image));
        assert_eq!(media_kind(Path::new("a.avif")), Some(MediaKind::Image));
        assert_eq!(media_kind(Path::new("a.heic")), Some(MediaKind::Image));
        assert_eq!(media_kind(Path::new("a.jxl")), Some(MediaKind::Image));
        assert_eq!(media_kind(Path::new("a.gif")), Some(MediaKind::Image));
        assert_eq!(media_kind(Path::new("a.bmp")), Some(MediaKind::Image));
        assert_eq!(media_kind(Path::new("a.ico")), Some(MediaKind::Image));
        assert_eq!(media_kind(Path::new("a.MP4")), Some(MediaKind::Video));
        assert_eq!(media_kind(Path::new("a.mkv")), Some(MediaKind::Video));
        assert_eq!(media_kind(Path::new("a.txt")), None);
    }

    #[test]
    fn next_and_previous_wrap_around() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join("a.jpg"));
        touch(&dir.path().join("b.jpg"));
        touch(&dir.path().join("c.jpg"));

        let mut navigator = FolderNavigator::new(dir.path()).unwrap();

        assert_eq!(navigator.next(), dir.path().join("b.jpg"));
        assert_eq!(navigator.next(), dir.path().join("c.jpg"));
        assert_eq!(navigator.next(), dir.path().join("a.jpg"));
        assert_eq!(navigator.previous(), dir.path().join("c.jpg"));
    }

    #[test]
    fn opening_a_file_starts_in_its_parent_folder() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join("a.jpg"));
        let target = dir.path().join("b.jpg");
        touch(&target);
        touch(&dir.path().join("c.jpg"));

        let mut navigator = FolderNavigator::new(&target).unwrap();

        assert_eq!(
            navigator.files,
            vec![
                dir.path().join("a.jpg"),
                dir.path().join("b.jpg"),
                dir.path().join("c.jpg"),
            ]
        );
        assert_eq!(navigator.current_path(), target);
        assert_eq!(navigator.next(), dir.path().join("c.jpg"));
        assert_eq!(navigator.previous(), target);
        assert_eq!(navigator.previous(), dir.path().join("a.jpg"));
    }

    #[test]
    fn unsupported_opened_file_is_included_at_sorted_position() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join("a.jpg"));
        let file = dir.path().join("notes.txt");
        touch(&file);
        touch(&dir.path().join("z.jpg"));

        let mut navigator = FolderNavigator::new(&file).unwrap();

        assert_eq!(
            navigator.files,
            vec![
                dir.path().join("a.jpg"),
                dir.path().join("notes.txt"),
                dir.path().join("z.jpg"),
            ]
        );
        assert_eq!(navigator.current_path(), file);
        assert_eq!(navigator.next(), dir.path().join("z.jpg"));
        assert_eq!(navigator.previous(), file);
    }

    #[test]
    fn unsupported_opened_file_is_inserted_in_natural_order() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join("img1.jpg"));
        touch(&dir.path().join("img10.jpg"));
        let file = dir.path().join("img2.cr3");
        touch(&file);

        let navigator = FolderNavigator::new(&file).unwrap();

        assert_eq!(
            navigator.files,
            vec![
                dir.path().join("img1.jpg"),
                dir.path().join("img2.cr3"),
                dir.path().join("img10.jpg"),
            ]
        );
        assert_eq!(navigator.current_path(), file);
    }

    #[test]
    fn unsupported_opened_file_stays_in_list_after_navigation() {
        let dir = tempdir().unwrap();
        let image = dir.path().join("a.jpg");
        touch(&image);
        let unsupported = dir.path().join("notes.txt");
        touch(&unsupported);

        let mut navigator = FolderNavigator::new(&unsupported).unwrap();

        assert_eq!(navigator.current_path(), unsupported);
        assert_eq!(navigator.next(), image);
        assert_eq!(navigator.previous(), unsupported);
        assert_eq!(navigator.files.len(), 2);
    }

    #[test]
    fn unsupported_file_alone_is_a_single_entry() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("notes.txt");
        touch(&file);

        let mut navigator = FolderNavigator::new(&file).unwrap();

        assert_eq!(navigator.files, vec![file.clone()]);
        assert_eq!(navigator.current_path(), file);
        assert_eq!(navigator.current_index, 0);
        assert!(!navigator.has_multiple_files());
        assert_eq!(navigator.next(), file);
        assert_eq!(navigator.previous(), file);
    }

    #[test]
    fn single_supported_file_starts_at_itself() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("clip.MP4");
        touch(&file);

        let mut navigator = FolderNavigator::new(&file).unwrap();

        assert_eq!(navigator.current_index, 0);
        assert!(!navigator.has_multiple_files());
        assert_eq!(navigator.next(), file);
        assert_eq!(navigator.previous(), file);
    }

    #[test]
    fn missing_path_errors() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("missing");

        let error = FolderNavigator::new(&missing).unwrap_err();

        assert!(matches!(
            error.downcast_ref::<AppError>(),
            Some(AppError::PathNotFound(path)) if path == &missing
        ));
    }

    #[test]
    fn folder_without_supported_files_errors() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join("notes.txt"));

        let error = FolderNavigator::new(dir.path()).unwrap_err();

        assert!(matches!(
            error.downcast_ref::<AppError>(),
            Some(AppError::NoSupportedFiles(path)) if path == dir.path()
        ));
    }
}
