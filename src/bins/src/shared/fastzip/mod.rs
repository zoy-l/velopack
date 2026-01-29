#![allow(dead_code)]

mod cloneable_seekable_reader;
mod progress_updater;
mod ripunzip;

use anyhow::Result;
use ripunzip::{UnzipEngine, UnzipOptions};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Compression level that should be used when compressing a file or data.
/// Current compression providers support only levels from 0 to 9, so these are the only ones being supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CompressionLevel(u8);

impl CompressionLevel {
    #[inline]
    pub const fn new(level: u8) -> Option<Self> {
        if level <= 9 {
            Some(Self(level))
        } else {
            None
        }
    }
    #[inline]
    pub const fn none() -> Self {
        Self(0)
    }
    #[inline]
    pub const fn fast() -> Self {
        Self(1)
    }
    #[inline]
    pub const fn balanced() -> Self {
        Self(6)
    }
    #[inline]
    pub const fn best() -> Self {
        Self(9)
    }
    #[inline]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl Default for CompressionLevel {
    fn default() -> Self {
        Self::balanced()
    }
}

/// A trait of types which wish to hear progress updates on the unzip.
pub trait UnzipProgressReporter: Sync {
    /// Extraction has begun on a file.
    fn extraction_starting(&self, _display_name: &str) {}
    /// Extraction has finished on a file.
    fn extraction_finished(&self, _display_name: &str) {}
    /// The total number of compressed bytes we expect to extract.
    fn total_bytes_expected(&self, _expected: u64) {}
    /// Some bytes of a file have been decompressed. This is probably
    /// the best way to display an overall progress bar. This should eventually
    /// add up to the number you're given using `total_bytes_expected`.
    /// The 'count' parameter is _not_ a running total - you must add up
    /// each call to this function into the running total.
    /// It's a bit unfortunate that we give compressed bytes rather than
    /// uncompressed bytes, but currently we can't calculate uncompressed
    /// bytes without downloading the whole zip file first, which rather
    /// defeats the point.
    fn bytes_extracted(&self, _count: u64) {}
}

/// A progress reporter which does nothing.
struct NullProgressReporter;

impl UnzipProgressReporter for NullProgressReporter {}

pub fn extract_to_directory<'b, P1: AsRef<Path>, P2: AsRef<Path>>(
    archive_file: P1,
    target_dir: P2,
    progress_reporter: Option<Box<dyn UnzipProgressReporter + Sync + 'b>>,
) -> Result<()> {
    let target_dir = target_dir.as_ref().to_path_buf();
    let file = File::open(archive_file)?;
    let engine = UnzipEngine::for_file(file)?;
    let null_progress = Box::new(NullProgressReporter {});
    let options = UnzipOptions {
        filename_filter: None,
        progress_reporter: progress_reporter.unwrap_or(null_progress),
        output_directory: Some(target_dir),
        password: None,
        single_threaded: false,
    };
    engine.unzip(options)?;
    Ok(())
}

pub fn compress_directory<'b, P1: AsRef<Path>, P2: AsRef<Path>>(target_dir: P1, output_file: P2, _level: CompressionLevel) -> Result<()> {
    let target_dir = target_dir.as_ref();
    let output_file = output_file.as_ref();

    let file = File::create(&output_file)?;
    let mut zip = zip::ZipWriter::new(file);

    let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Deflated).unix_permissions(0o755);

    let walker = WalkDir::new(target_dir).into_iter();
    for entry in walker {
        let entry = entry?;
        let path = entry.path();

        let name = path.strip_prefix(target_dir)?.to_string_lossy();
        let name = name.replace("\\", "/"); // zip spec requires forward slashes

        if path.is_dir() {
            if !name.is_empty() {
                zip.add_directory(name, options)?;
            }
        } else {
            zip.start_file(name, options)?;
            let mut f = File::open(path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        }
    }
    zip.finish()?;
    Ok(())
}

pub fn enumerate_files_relative<P: AsRef<Path>>(dir: P) -> Vec<PathBuf> {
    WalkDir::new(&dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.path().strip_prefix(&dir).map(|p| p.to_path_buf()))
        .filter_map(|entry| entry.ok())
        .collect()
}
