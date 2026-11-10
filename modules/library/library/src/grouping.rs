// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The grouping rule: which files of a folder form one asset (LIBRARY-001).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use domain::{AssetFileRole, RAW_EXTENSIONS, VIDEO_EXTENSIONS};

/// The rendered extensions that pair with a RAW, in upper case (LIBRARY-001).
const RENDERED_EXTENSIONS: [&str; 7] = ["JPG", "JPEG", "HEIF", "HEIC", "HIF", "TIF", "TIFF"];

const XMP_EXTENSION: &str = "xmp";
const THM_EXTENSION: &str = "THM";

/// A file of a folder, as the grouping takes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderFile {
    /// The file's name within its folder.
    pub name: String,
    /// The file's size in bytes.
    pub size_bytes: u64,
}

/// One file of an asset, with its role (glossary: Asset file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetFile {
    /// Where the file is.
    pub path: PathBuf,
    /// What the file is to its asset.
    pub role: AssetFileRole,
    /// The file's size in bytes.
    pub size_bytes: u64,
}

impl AssetFile {
    fn name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
    }
}

/// An image or video together with its files (glossary: Asset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    base_name: String,
    files: Vec<AssetFile>,
}

impl Asset {
    /// The base name its files share: the file name up to the last `.`.
    #[must_use]
    pub fn base_name(&self) -> &str {
        &self.base_name
    }

    /// Its files: the RAW first, then the other media files and then the sidecars,
    /// each by name.
    #[must_use]
    pub fn files(&self) -> &[AssetFile] {
        &self.files
    }

    /// The file with `role`, if the asset has one; for [`AssetFileRole::Sidecar`],
    /// the first sidecar.
    #[must_use]
    pub fn file(&self, role: AssetFileRole) -> Option<&AssetFile> {
        self.files.iter().find(|file| file.role == role)
    }

    /// The path that names the asset: that of its first media file, which no other
    /// asset has.
    #[must_use]
    pub fn id(&self) -> &Path {
        self.files
            .first()
            .map_or_else(|| Path::new(""), |file| file.path.as_path())
    }

    /// The asset's media files: those that are not sidecars.
    pub fn media(&self) -> impl Iterator<Item = &AssetFile> {
        self.files
            .iter()
            .filter(|file| file.role != AssetFileRole::Sidecar)
    }

    /// The XMP sidecars of the asset: the own ones and a shared `<base>.xmp`.
    pub fn xmp_sidecars(&self) -> impl Iterator<Item = &AssetFile> {
        self.files.iter().filter(|file| {
            file.role == AssetFileRole::Sidecar && described_name(file.name()).is_some()
        })
    }

    /// The shared sidecar `<base>.xmp`, if the asset has one.
    #[must_use]
    pub fn shared_sidecar(&self) -> Option<&AssetFile> {
        self.xmp_sidecars()
            .find(|sidecar| described_name(sidecar.name()) == Some(self.base_name.as_str()))
    }

    /// The own XMP sidecars of `media`, such as `DSCF1234.RAF.xmp`, but not a shared
    /// `DSCF1234.xmp`.
    pub fn own_sidecars<'a>(&'a self, media: &'a AssetFile) -> impl Iterator<Item = &'a AssetFile> {
        self.xmp_sidecars()
            .filter(move |sidecar| described_name(sidecar.name()) == Some(media.name()))
    }
}

/// A base name in a folder whose files cannot form one asset, because more than one
/// of them is a RAW or more than one is rendered (LIBRARY-001.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmbiguousBaseName {
    /// The folder.
    pub folder: PathBuf,
    /// The base name.
    pub base_name: String,
}

/// The assets of one folder, and the base names that did not group.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Grouping {
    /// The assets, sorted by base name and then by the name of their first file.
    pub assets: Vec<Asset>,
    /// The base names whose files became an asset each.
    pub ambiguous: Vec<AmbiguousBaseName>,
}

/// Groups the `files` of `folder` into assets by their names alone (LIBRARY-001).
///
/// Files with the same base name form one asset when at most one is a RAW and at most
/// one is rendered. A video is an asset of its own, with a `.THM` of the same base
/// name. An `.xmp` sidecar joins the asset of the file it is named after, or as a
/// shared `<base>.xmp` the asset of that base name. Files of other types are not part
/// of the library.
#[must_use]
pub fn group_files(folder: &Path, files: &[FolderFile]) -> Grouping {
    let mut grouping = Grouping::default();
    for (base_name, media) in media_by_base_name(files) {
        let (assets, ambiguous) = assets_of_base_name(folder, &base_name, media);
        if ambiguous {
            grouping.ambiguous.push(AmbiguousBaseName {
                folder: folder.to_path_buf(),
                base_name: base_name.clone(),
            });
        }
        grouping.assets.extend(assets);
    }
    add_sidecars(folder, files, &mut grouping.assets);
    grouping
}

fn media_by_base_name(files: &[FolderFile]) -> BTreeMap<String, Vec<(AssetFileRole, &FolderFile)>> {
    let mut by_base_name: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for file in files {
        let Some((base_name, extension)) = file.name.rsplit_once('.') else {
            continue;
        };
        if let Some(role) = media_role(extension) {
            by_base_name
                .entry(base_name.to_owned())
                .or_default()
                .push((role, file));
        }
    }
    for media in by_base_name.values_mut() {
        media.sort_by(|(_, a), (_, b)| a.name.cmp(&b.name));
    }
    by_base_name
}

/// The assets of the media files with one base name, and whether their images were
/// ambiguous.
fn assets_of_base_name(
    folder: &Path,
    base_name: &str,
    media: Vec<(AssetFileRole, &FolderFile)>,
) -> (Vec<Asset>, bool) {
    let count = |role| media.iter().filter(|(r, _)| *r == role).count();
    let ambiguous = count(AssetFileRole::Raw) > 1 || count(AssetFileRole::Rendered) > 1;
    let (videos, images): (Vec<_>, Vec<_>) = media
        .into_iter()
        .partition(|(role, _)| *role == AssetFileRole::Video);
    let mut groups: Vec<Vec<_>> = videos.into_iter().map(|video| vec![video]).collect();
    if ambiguous {
        groups.extend(images.into_iter().map(|image| vec![image]));
    } else if !images.is_empty() {
        groups.push(images);
    }
    let assets = groups
        .into_iter()
        .map(|group| asset(folder, base_name, group))
        .collect();
    (assets, ambiguous)
}

fn asset(folder: &Path, base_name: &str, mut group: Vec<(AssetFileRole, &FolderFile)>) -> Asset {
    // The RAW comes first, so it names the asset and wins ties between sidecars.
    group.sort_by_key(|(role, _)| *role != AssetFileRole::Raw);
    Asset {
        base_name: base_name.to_owned(),
        files: group
            .into_iter()
            .map(|(role, file)| asset_file(folder, file, role))
            .collect(),
    }
}

fn add_sidecars(folder: &Path, files: &[FolderFile], assets: &mut [Asset]) {
    let mut sidecars: Vec<&FolderFile> = files
        .iter()
        .filter(|file| described_name(&file.name).is_some() || is_thm(&file.name))
        .collect();
    sidecars.sort_by(|a, b| a.name.cmp(&b.name));
    for sidecar in sidecars {
        if let Some(asset) = owner_of_sidecar(&sidecar.name, assets) {
            let file = asset_file(folder, sidecar, AssetFileRole::Sidecar);
            asset.files.push(file);
        }
    }
}

fn owner_of_sidecar<'a>(name: &str, assets: &'a mut [Asset]) -> Option<&'a mut Asset> {
    let index = if let Some(described) = described_name(name) {
        let own = assets
            .iter()
            .position(|asset| asset.media().any(|file| file.name() == described));
        own.or_else(|| shared_owner(described, assets, |_| true))
    } else {
        let base_name = name.rsplit_once('.').map(|(base, _)| base)?;
        shared_owner(base_name, assets, is_video)
    };
    assets.get_mut(index?)
}

/// The one asset with `base_name` that `fits`; none when the base name is ambiguous.
fn shared_owner(base_name: &str, assets: &[Asset], fits: impl Fn(&Asset) -> bool) -> Option<usize> {
    let mut matching = assets
        .iter()
        .enumerate()
        .filter(|(_, asset)| asset.base_name == base_name && fits(asset));
    let (index, _) = matching.next()?;
    matching.next().is_none().then_some(index)
}

/// The name of the file a sidecar `<name>.xmp` describes, or `None` for a file that
/// is no XMP sidecar.
fn described_name(name: &str) -> Option<&str> {
    let (described, extension) = name.rsplit_once('.')?;
    extension
        .eq_ignore_ascii_case(XMP_EXTENSION)
        .then_some(described)
}

fn is_thm(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case(THM_EXTENSION))
}

fn is_video(asset: &Asset) -> bool {
    asset.file(AssetFileRole::Video).is_some()
}

fn media_role(extension: &str) -> Option<AssetFileRole> {
    let lists: [(&[&str], AssetFileRole); 3] = [
        (&RAW_EXTENSIONS, AssetFileRole::Raw),
        (&RENDERED_EXTENSIONS, AssetFileRole::Rendered),
        (&VIDEO_EXTENSIONS, AssetFileRole::Video),
    ];
    lists
        .into_iter()
        .find(|(extensions, _)| {
            extensions
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        })
        .map(|(_, role)| role)
}

fn asset_file(folder: &Path, file: &FolderFile, role: AssetFileRole) -> AssetFile {
    AssetFile {
        path: folder.join(&file.name),
        role,
        size_bytes: file.size_bytes,
    }
}
