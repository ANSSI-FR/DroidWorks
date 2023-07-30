//! `DroidWorks` sub-crate to manage files and assets that are contained in
//! an Android application.

mod helpers;

pub mod errors;

use crate::errors::{PackageError, PackageResult};
use base64::{engine::general_purpose as b64, Engine};
use dw_resources::{manifest, nsc, resources};
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::FileOptions;
use zip::{result::ZipError, CompressionMethod, ZipArchive, ZipWriter};

/// A package represents an Android application and provides accessors to
/// easily find assets such as application resources or code.
#[derive(Debug)]
pub struct Package {
    pub(crate) name: String,
    pub(crate) dexs_path: Vec<PathBuf>,
    pub(crate) manifest_path: Option<PathBuf>,
    pub(crate) nsc_path: Option<PathBuf>,
    pub(crate) resources_path: Option<PathBuf>,
    pub(crate) files: BTreeMap<PathBuf, FileItem>,
}

impl Package {
    /// Open the given filename as an Android application and returns the
    /// corresponding [`Package`] object.
    pub fn open<P: AsRef<Path>>(path: P) -> PackageResult<Self> {
        Options::default().open(path)
    }

    /// Returns an iterator over file names that are contained in the package.
    pub fn iter_filenames(&self) -> impl Iterator<Item = &Path> {
        self.files.keys().map(PathBuf::as_path)
    }

    /// Returns an iterator over file names and sizes that are contained in
    /// the package.
    pub fn iter_filenames_with_size(&self) -> impl Iterator<Item = (&Path, usize)> {
        self.files
            .iter()
            .map(|(name, file_item)| (PathBuf::as_path(name), file_item.raw.len()))
    }

    /// Due to a limitation in the number of methods that can be stored in a Dex
    /// file, Android application can have their code split between several Dex files.
    /// This method returns an iterator over all Dex object that are contained in
    /// the package.
    pub fn iter_dexs(&self) -> impl Iterator<Item = &dw_dex::Dex> {
        self.dexs_path
            .iter()
            .map(move |path| match self.files.get(path) {
                Some(FileItem {
                    content: FileContent::Dex(d),
                    ..
                }) => d,
                _ => unreachable!(),
            })
    }

    pub fn modify_dexs(&mut self) {
        for path in self.dexs_path.clone() {
            match self.files.get_mut(&path) {
                Some(fitem) => fitem.modify(),
                _ => unreachable!(),
            }
        }
    }

    pub fn get(&self, asset: &PathBuf) -> PackageResult<&[u8]> {
        match self.files.get(asset) {
            None => Err(PackageError::Zip(ZipError::FileNotFound)),
            Some(fileitem) => {
                if fileitem.modified {
                    let filename = asset.as_path().to_str().unwrap().to_string();
                    return Err(PackageError::FileHasBeenModified(filename));
                }
                Ok(&fileitem.raw)
            }
        }
    }

    /// Extract the given asset from the package in the `output` file.
    pub fn extract_file<P: AsRef<Path>>(&self, asset: &PathBuf, output: P) -> PackageResult<()> {
        match self.files.get(asset) {
            None => Err(PackageError::Zip(ZipError::FileNotFound)),
            Some(fileitem) => {
                if fileitem.modified {
                    let filename = asset.as_path().to_str().unwrap().to_string();
                    return Err(PackageError::FileHasBeenModified(filename));
                }

                let mut file = File::create(output)?;
                file.write_all(&fileitem.raw)?;
                Ok(())
            }
        }
    }

    /// Extract the given asset and returns its content in a base64 encoded string.
    pub fn base64_file(&self, asset: &PathBuf) -> PackageResult<String> {
        match self.files.get(asset) {
            None => Err(PackageError::Zip(ZipError::FileNotFound)),
            Some(fileitem) => {
                if fileitem.modified {
                    let filename = asset.as_path().to_str().unwrap().to_string();
                    Err(PackageError::FileHasBeenModified(filename))
                } else {
                    Ok(b64::STANDARD.encode(&fileitem.raw))
                }
            }
        }
    }

    /// Remove the given asset from the package.
    pub fn remove_file(&mut self, asset: &PathBuf) -> PackageResult<()> {
        match self.files.remove(asset) {
            None => Err(PackageError::Zip(ZipError::FileNotFound)),
            Some(_) => Ok(()),
        }
    }

    /// Replace the content of the given asset by a new `content`.
    pub fn replace_file_other(&mut self, asset: PathBuf, content: Vec<u8>) -> PackageResult<()> {
        match self.files.remove(&asset) {
            None => Err(PackageError::Zip(ZipError::FileNotFound)),
            Some(old) => {
                self.files
                    .insert(asset, FileItem::new_other(content, old.compression));
                Ok(())
            }
        }
    }

    pub fn replace_file_nsc(&mut self, content: Vec<u8>) -> PackageResult<()> {
        let nsc_path = if let Some(nsc_path) = &self.nsc_path {
            Ok(nsc_path.clone())
        } else {
            Err(PackageError::Zip(ZipError::FileNotFound))
        }?;
        self.replace_file_other(nsc_path.clone(), content)?;
        self.set_nsc_path(nsc_path)
    }

    pub fn insert_file(&mut self, asset: PathBuf, content: Vec<u8>) -> PackageResult<()> {
        if self.files.get(&asset).is_some() {
            return Err(PackageError::Zip(ZipError::InvalidArchive(
                "file already exists",
            )));
        }
        self.files.insert(
            asset,
            FileItem::new_other(content, CompressionMethod::Stored),
        );
        Ok(())
    }

    /// Return a set that contains strings that are referenced in all Dex files
    /// that are contained in the package.
    pub fn dexs_strings(&self) -> PackageResult<BTreeSet<String>> {
        let mut strings = BTreeSet::new();

        for dex in self.iter_dexs() {
            for sid in dex.iter_string_ids() {
                strings.insert(sid.to_string(dex)?);
            }
        }

        Ok(strings)
    }

    /// Return the Android Manifest object of the package.
    #[must_use]
    pub fn manifest(&self) -> Option<&manifest::Manifest> {
        let path = self.manifest_path.as_ref()?;
        match self.files.get(path) {
            Some(FileItem {
                content: FileContent::Manifest(m),
                ..
            }) => Some(m),
            _ => unreachable!(),
        }
    }

    #[must_use]
    pub fn network_security_config(&self) -> Option<&nsc::NetworkSecurityConfig> {
        let path = self.nsc_path.as_ref()?;
        match self.files.get(path) {
            Some(FileItem {
                content: FileContent::NetworkSecurityConfig(nsc),
                ..
            }) => Some(nsc),
            _ => unreachable!(),
        }
    }

    /// Return the main Android Resources object of the package.
    #[must_use]
    pub fn resources(&self) -> Option<&resources::Resources> {
        let path = self.resources_path.as_ref()?;
        match self.files.get(path) {
            Some(FileItem {
                content: FileContent::Resources(r),
                ..
            }) => Some(r),
            _ => unreachable!(),
        }
    }

    /// Returns a mutable reference to manifest structure, and mark the manifest
    /// as modified for future export.
    /// If manifest modification is not intended, use method `manifest` instead,
    /// this there will be no need to recompute all manifest tables when exporting
    /// the whole package.
    pub fn manifest_mut(&mut self) -> Option<&mut manifest::Manifest> {
        let path = self.manifest_path.as_ref()?;
        match self.files.get_mut(path) {
            Some(FileItem {
                content: FileContent::Manifest(m),
                modified,
                ..
            }) => {
                *modified = true;
                Some(m)
            }
            _ => unreachable!(),
        }
    }

    pub fn set_nsc_path(&mut self, path: PathBuf) -> PackageResult<()> {
        match self.files.get_mut(&path) {
            None => return Err(PackageError::Zip(ZipError::FileNotFound)),
            Some(fileitem) => {
                if fileitem.modified {
                    let filename = path.as_path().to_str().unwrap().to_string();
                    return Err(PackageError::FileHasBeenModified(filename));
                }
                fileitem.convert_to_nsc()?;
            }
        }
        self.nsc_path = Some(path);
        Ok(())
    }

    pub fn save<P: AsRef<Path>>(&self, path: P, clean_signature: bool) -> PackageResult<()> {
        log::trace!("preparing zip file {:?}...", path.as_ref());
        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);

        for (path, fileitem) in &self.files {
            log::trace!("adding {:?} to zip", path);
            let path = path.to_str().unwrap().to_string();
            let drop_it = clean_signature
                && matches!(
                    path.as_str(),
                    "META-INF/CERT.RSA" | "META-INF/CERT.SF" | "META-INF/MANIFEST.MF"
                );
            if !drop_it {
                let options = FileOptions::default().compression_method(fileitem.compression);
                zip.start_file(&path, options)?;
                if fileitem.keep_it {
                    if fileitem.modified {
                        match &fileitem.content {
                            FileContent::Dex(dex) => {
                                let buf = dw_dex::write(dex, true)?;
                                zip.write_all(&buf)?;
                            }
                            FileContent::Manifest(manifest) => {
                                let buf = manifest::write(manifest)?;
                                zip.write_all(&buf)?;
                            }
                            FileContent::NetworkSecurityConfig(nsc) => {
                                let buf = nsc::write(nsc)?;
                                zip.write_all(&buf)?;
                            }
                            FileContent::Resources(resources) => {
                                let buf = resources::write(resources)?;
                                zip.write_all(&buf)?;
                            }
                            FileContent::Other => {
                                // 'other' items cannot be modified
                                unreachable!();
                            }
                        }
                    } else {
                        zip.write_all(&fileitem.raw)?;
                    }
                }
            }
        }

        log::trace!("finishing zip file");
        zip.finish()?;
        Ok(())
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{} files:", self.name)?;
        for path in self.files.keys() {
            writeln!(f, "  - {path:?}")?;
        }
        Ok(())
    }
}

/// Options to select which kind of asset is actually parsed when opening
/// an Android [package](Package).
#[derive(Debug)]
pub struct Options {
    parse_dex: bool,
    parse_manifest: bool,
    parse_resources: bool,
}

/// Default values enable dex parsing, manifest parsing and resources parsing.
impl Default for Options {
    fn default() -> Self {
        Self {
            parse_dex: true,
            parse_manifest: true,
            parse_resources: true,
        }
    }
}

impl Options {
    /// Parse only Dex files when opening a package and passing this [`Option`].
    #[must_use]
    pub const fn dex_only() -> Self {
        Self {
            parse_dex: true,
            parse_manifest: false,
            parse_resources: false,
        }
    }

    /// Parse only Manifest file when opening a package and passing this [`Option`].
    #[must_use]
    pub const fn manifest_only() -> Self {
        Self {
            parse_dex: false,
            parse_manifest: true,
            parse_resources: false,
        }
    }

    #[must_use]
    pub const fn resources_only() -> Self {
        Self {
            parse_dex: false,
            parse_manifest: false,
            parse_resources: true,
        }
    }

    #[must_use]
    pub const fn dont_parse_dex(self) -> Self {
        Self {
            parse_dex: false,
            ..self
        }
    }

    #[must_use]
    pub const fn dont_parse_manifest(self) -> Self {
        Self {
            parse_manifest: false,
            parse_resources: false,
            ..self
        }
    }

    #[must_use]
    pub const fn dont_parse_resources(self) -> Self {
        Self {
            parse_resources: false,
            ..self
        }
    }

    pub fn open<P: AsRef<Path>>(self, path: P) -> PackageResult<Package> {
        enum Task<T> {
            Dex(PathBuf, CompressionMethod, T),
            Manifest(PathBuf, CompressionMethod, T),
            Resources(PathBuf, CompressionMethod, T),
        }

        let package_name = path
            .as_ref()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let file = File::open(path)?;
        let mut zip = ZipArchive::new(file)?;
        let mut package = Package {
            name: package_name,
            dexs_path: Vec::new(),
            manifest_path: None,
            nsc_path: None,
            resources_path: None,
            files: BTreeMap::new(),
        };

        let mut tasks = Vec::new();
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            let pathbuf = PathBuf::from(file.name());
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            if self.parse_dex && helpers::is_dex(file.name()) {
                tasks.push(Task::Dex(pathbuf, file.compression(), buffer));
            } else if self.parse_manifest && helpers::is_manifest(file.name()) {
                tasks.push(Task::Manifest(pathbuf, file.compression(), buffer));
            } else if self.parse_resources && helpers::is_resources(file.name()) {
                tasks.push(Task::Resources(pathbuf, file.compression(), buffer));
            } else {
                package
                    .files
                    .insert(pathbuf, FileItem::new_other(buffer, file.compression()));
            }
        }

        let tasks_results: Vec<Task<FileItem>> = tasks
            .into_par_iter()
            .map(|task: Task<Vec<u8>>| -> PackageResult<Task<FileItem>> {
                match task {
                    Task::Dex(filename, compression, buf) => {
                        let dex = dw_dex::parse(&buf)?;
                        Ok(Task::Dex(
                            filename,
                            compression,
                            FileItem::new_dex(buf, compression, dex),
                        ))
                    }
                    Task::Manifest(filename, compression, buf) => {
                        let manifest = manifest::parse(&buf)?;
                        Ok(Task::Manifest(
                            filename,
                            compression,
                            FileItem::new_manifest(buf, compression, manifest),
                        ))
                    }
                    Task::Resources(filename, compression, buf) => {
                        let resources = resources::parse(&buf)?;
                        Ok(Task::Resources(
                            filename,
                            compression,
                            FileItem::new_resources(buf, compression, resources),
                        ))
                    }
                }
            })
            .collect::<PackageResult<Vec<Task<FileItem>>>>()?;

        tasks_results.into_iter().for_each(|result| match result {
            Task::Dex(filename, _, item) => {
                package.dexs_path.push(filename.clone());
                package.files.insert(filename, item);
            }
            Task::Manifest(filename, _, item) => {
                package.manifest_path = Some(filename.clone());
                package.files.insert(filename, item);
            }
            Task::Resources(filename, _, item) => {
                package.resources_path = Some(filename.clone());
                package.files.insert(filename, item);
            }
        });

        Ok(package)
    }
}

#[derive(Debug)]
struct FileItem {
    raw: Vec<u8>,
    compression: CompressionMethod,
    modified: bool,
    keep_it: bool,
    content: FileContent,
}

impl FileItem {
    fn new_dex(raw: Vec<u8>, compression: CompressionMethod, dex: dw_dex::Dex) -> Self {
        Self {
            raw,
            compression,
            modified: false,
            keep_it: true,
            content: FileContent::Dex(dex),
        }
    }

    fn new_manifest(
        raw: Vec<u8>,
        compression: CompressionMethod,
        manifest: manifest::Manifest,
    ) -> Self {
        Self {
            raw,
            compression,
            modified: false,
            keep_it: true,
            content: FileContent::Manifest(manifest),
        }
    }

    fn new_resources(
        raw: Vec<u8>,
        compression: CompressionMethod,
        resources: resources::Resources,
    ) -> Self {
        Self {
            raw,
            compression,
            modified: false,
            keep_it: true,
            content: FileContent::Resources(resources),
        }
    }

    fn new_other(raw: Vec<u8>, compression: CompressionMethod) -> Self {
        Self {
            raw,
            compression,
            modified: false,
            keep_it: true,
            content: FileContent::Other,
        }
    }

    fn convert_to_nsc(&mut self) -> PackageResult<()> {
        let nsc = nsc::parse(&self.raw)?;
        self.content = FileContent::NetworkSecurityConfig(nsc);
        Ok(())
    }

    fn modify(&mut self) {
        self.modified = true;
    }
}

#[derive(Debug)]
pub(crate) enum FileContent {
    Dex(dw_dex::Dex),
    Manifest(manifest::Manifest),
    NetworkSecurityConfig(nsc::NetworkSecurityConfig),
    Resources(resources::Resources),
    Other,
}
