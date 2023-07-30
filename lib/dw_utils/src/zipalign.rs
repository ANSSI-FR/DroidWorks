//! Zip alignment functions.

use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::Path;
use thiserror::Error;

/// An alias for result that can be a [`ZipError`].
pub type ZipResult<T> = Result<T, ZipError>;

/// Zip error.
#[derive(Debug, Error)]
pub enum ZipError {
    /// Error that can be returned when trying to manipulate zip file in-place
    /// (i.e. input and output files are the same).
    #[error("Input an output can't be same file")]
    FilenamesConflict,

    /// Error that can be returned when trying to overwrite existing file.
    #[error("Output file '{0}' exists")]
    OutputExists(String),

    /// Error that can be returned when failing to open a file as a zip archive.
    #[error("Unable to open '{0}' as zip archive")]
    Open(String),

    /// Error that can be returned when a failure occurs generating the new zip archive.
    #[error("zipalign: failed rewriting zip file")]
    Rewrite,
}

/// Open `infilename` zip file, align the zip structure to `alignment`,
/// and generates `outfilename` resulting zip file.
pub fn process(
    infilename: &str,
    outfilename: &str,
    alignment: u64,
    force: bool,
    zopfli: bool,
    page_align_shared_libs: bool,
) -> ZipResult<()> {
    /* this mode isn't supported -- do a trivial check */
    if infilename == outfilename {
        return Err(ZipError::FilenamesConflict);
    }

    /* don't overwrite existing unless given permission */
    if !force && Path::new(outfilename).is_file() {
        return Err(ZipError::OutputExists(outfilename.to_string()));
    }

    let infile = File::open(infilename).map_err(|_| ZipError::Open(infilename.to_string()))?;
    let zin = zip::ZipArchive::new(infile).map_err(|_| ZipError::Open(infilename.to_string()))?;

    let outfile = File::create(outfilename).map_err(|_| ZipError::Open(outfilename.to_string()))?;
    let zout = zip::ZipWriter::new(outfile);

    copy_and_align(zin, zout, alignment, zopfli, page_align_shared_libs)
}

/// Returns whether or not the zip structure of `filename` respect the `alignment`.
pub fn verify(
    filename: &str,
    alignment: u64,
    verbose: bool,
    page_align_shared_libs: bool,
) -> ZipResult<bool> {
    if verbose {
        println!("verifying alignment of {filename} ({alignment})...");
    }

    let file = File::open(filename).map_err(|_| ZipError::Open(filename.to_string()))?;
    let mut zipfile =
        zip::ZipArchive::new(file).map_err(|_| ZipError::Open(filename.to_string()))?;
    let mut found_bad = false;

    for i in 0..zipfile.len() {
        let entry = zipfile.by_index(i).expect("zip file");
        if entry.compression() == zip::CompressionMethod::Stored {
            let offset = entry.data_start();
            let align_to = get_alignment(page_align_shared_libs, alignment, &entry);
            if offset % align_to == 0 {
                if verbose {
                    println!("{} {} (OK)", offset, entry.name());
                }
                continue;
            }
            found_bad = true;
            if verbose {
                println!("{} {} (BAD - {})", offset, entry.name(), offset % align_to);
            }
        }
        if verbose {
            println!("{} {} (OK - compressed)", entry.data_start(), entry.name());
        }
    }

    if verbose {
        println!(
            "Verification {}",
            if found_bad { "FAILED" } else { "succesful" }
        );
    }

    Ok(found_bad)
}

fn copy_and_align<R: Read + Seek, W: Write + Seek>(
    mut zin: zip::ZipArchive<R>,
    mut zout: zip::ZipWriter<W>,
    alignment: u64,
    zopfli: bool,
    page_align_shared_libs: bool,
) -> ZipResult<()> {
    for i in 0..zin.len() {
        let mut entry = zin.by_index(i).expect("zip entry");

        if entry.compression() == zip::CompressionMethod::Stored {
            let align_to: u64 = get_alignment(page_align_shared_libs, alignment, &entry);
            zout.start_file_aligned(
                entry.name(),
                zip::write::FileOptions::default().compression_method(entry.compression()),
                align_to as u16,
            )
            .map_err(|_| ZipError::Rewrite)?;
            let mut read_buf = Vec::new();
            entry
                .read_to_end(&mut read_buf)
                .map_err(|_| ZipError::Rewrite)?;
            zout.write(&read_buf).map_err(|_| ZipError::Rewrite)?;
        } else {
            /* copy the entry without padding */
            if zopfli {
                zout.start_file(
                    entry.name(),
                    zip::write::FileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated),
                )
                .map_err(|_| ZipError::Rewrite)?;
                let mut read_buf = Vec::new();
                entry
                    .read_to_end(&mut read_buf)
                    .map_err(|_| ZipError::Rewrite)?;
                zout.write(&read_buf).map_err(|_| ZipError::Rewrite)?;
            } else {
                zout.raw_copy_file(entry).map_err(|_| ZipError::Rewrite)?;
            }
        }
    }

    zout.finish().map_err(|_| ZipError::Rewrite)?;
    Ok(())
}

fn get_alignment(
    page_align_shared_libs: bool,
    default_alignment: u64,
    entry: &zip::read::ZipFile,
) -> u64 {
    const K_PAGE_ALIGNMENT: u64 = 4096;

    if !page_align_shared_libs {
        return default_alignment;
    }

    if let Some(ext) = Path::new(entry.name()).extension() {
        if ext == "so" {
            return K_PAGE_ALIGNMENT;
        }
    }

    default_alignment
}
