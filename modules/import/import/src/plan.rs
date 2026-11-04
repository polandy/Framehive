// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import plan: where each file of an import goes and what it is called
//! (IMPORT-001).

use domain::{AssetFileRole, CaptureTime, LocalDateTime};
use naming::{
    AssetDate, AssetValues, CompatibilityMode, DateSource, FileTimes, NameTemplate, RenderError,
    asset_date,
};

use crate::JobCode;

/// One file of an asset, as the import finds it on the source (glossary: Asset file).
///
/// Pure data: the caller reads the times from the file and applies camera time
/// correction before the plan is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetFile {
    /// The original file name, with its extension, e.g. `DSC_0042.RAF`.
    pub file_name: String,
    /// The role of the file in its asset.
    pub role: AssetFileRole,
    /// The capture time, after camera time correction, if the file records one.
    pub capture_time: Option<CaptureTime>,
    /// The file modification time.
    pub modification_time: LocalDateTime,
}

/// What the import plan decided for one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    /// The original file name.
    pub file_name: String,
    /// The path relative to the import destination, with `/` between the segments, or
    /// why this file cannot be named.
    pub target: Result<String, RenderError>,
    /// The time of the file's asset, in local time, as `{date:…}` renders it.
    pub date: LocalDateTime,
    /// Where the time of the file's asset comes from.
    pub date_source: DateSource,
}

impl PlannedFile {
    /// Whether no file of the asset had a capture time, so the user should see that
    /// the file is dated by a modification time (IMPORT-001, "Capture time").
    #[must_use]
    pub fn is_dated_by_modification_time(&self) -> bool {
        self.date_source == DateSource::ModificationTime
    }
}

/// A pure description of which file goes where, with what name (glossary: Import plan).
///
/// Every file of one asset is rendered from the same values (IMPORT-001, "Assets"). A
/// file that cannot be named is marked with the reason; the other files are planned
/// normally. Name conflicts are not part of the plan yet (IMPORT-003).
///
/// ```
/// use domain::{AssetFileRole, LocalDateTime};
/// use import::{AssetFile, ImportPlan};
/// use naming::{CompatibilityMode, NameTemplate};
///
/// let captured = LocalDateTime::new(2026, 10, 17, 14, 5, 9)?;
/// let raw = AssetFile {
///     file_name: "DSC_0042.RAF".to_owned(),
///     role: AssetFileRole::Raw,
///     capture_time: Some(captured.into()),
///     modification_time: captured,
/// };
/// let template = NameTemplate::parse("{date:%Y-%m-%d}/{name}.{ext}")?;
/// let import_time = LocalDateTime::new(2026, 10, 31, 9, 0, 0)?;
///
/// let plan = ImportPlan::new(&[vec![raw]], &template, CompatibilityMode::On, import_time, None, 1);
///
/// assert_eq!(plan.files()[0].target.as_deref(), Ok("2026-10-17/DSC_0042.RAF"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPlan {
    files: Vec<PlannedFile>,
}

impl ImportPlan {
    /// Makes the plan for already grouped assets (grouping, LIBRARY-001): each inner
    /// list holds the files of one asset.
    ///
    /// `import_time` is the time the import was queued, in the machine's local time.
    /// `job_code` gives `{job}` (IMPORT-006); without one, a template with `{job}` marks
    /// every file as not nameable. The assets are numbered in shooting order from
    /// `first_sequence_number` on, for the sequence token (IMPORT-002, "Order"). An
    /// asset without files adds nothing to the plan.
    #[must_use]
    pub fn new(
        assets: &[Vec<AssetFile>],
        template: &NameTemplate,
        mode: CompatibilityMode,
        import_time: LocalDateTime,
        job_code: Option<&JobCode>,
        first_sequence_number: u32,
    ) -> Self {
        let shared = SharedValues {
            template,
            mode,
            import_time,
            job_code,
        };
        let dated: Vec<Option<DatedAsset<'_>>> =
            assets.iter().map(|files| dated_asset(files)).collect();
        let numbers = shooting_order_numbers(&dated, first_sequence_number);
        let files = assets
            .iter()
            .zip(&dated)
            .zip(numbers)
            .filter_map(|((files, dated), number)| {
                Some(plan_asset(files, dated.as_ref()?, number, &shared))
            })
            .flatten()
            .collect();
        ImportPlan { files }
    }

    /// The number of assets the plan numbers: one sequence number each (IMPORT-002).
    #[must_use]
    pub fn asset_count(assets: &[Vec<AssetFile>]) -> usize {
        assets.iter().filter(|files| !files.is_empty()).count()
    }

    /// One planned file per input file, in the order of the input.
    #[must_use]
    pub fn files(&self) -> &[PlannedFile] {
        &self.files
    }
}

/// What every asset of one plan is rendered with.
struct SharedValues<'a> {
    template: &'a NameTemplate,
    mode: CompatibilityMode,
    import_time: LocalDateTime,
    job_code: Option<&'a JobCode>,
}

/// The time and base name of an asset with files.
struct DatedAsset<'a> {
    date: AssetDate,
    base_name: &'a str,
}

fn dated_asset(files: &[AssetFile]) -> Option<DatedAsset<'_>> {
    let times: Vec<FileTimes> = files.iter().map(file_times).collect();
    // `asset_date` is `None` only for an asset without files, which plans nothing.
    Some(DatedAsset {
        date: asset_date(&times)?,
        base_name: base_name(files)?,
    })
}

/// The sequence number of each asset: in shooting order, by time and then by base
/// name byte by byte (IMPORT-002, "Order"). Assets without files get none.
fn shooting_order_numbers(dated: &[Option<DatedAsset<'_>>], first: u32) -> Vec<u32> {
    let mut order: Vec<(usize, &DatedAsset<'_>)> = dated
        .iter()
        .enumerate()
        .filter_map(|(index, asset)| Some((index, asset.as_ref()?)))
        .collect();
    order.sort_by(|(_, a), (_, b)| {
        (a.date.time, a.base_name.as_bytes()).cmp(&(b.date.time, b.base_name.as_bytes()))
    });
    let mut numbers = vec![0; dated.len()];
    let mut next = first;
    for (index, _) in order {
        if let Some(number) = numbers.get_mut(index) {
            *number = next;
        }
        next = next.saturating_add(1);
    }
    numbers
}

fn plan_asset(
    files: &[AssetFile],
    asset: &DatedAsset<'_>,
    sequence_number: u32,
    shared: &SharedValues<'_>,
) -> Vec<PlannedFile> {
    let values = AssetValues {
        base_name: asset.base_name.to_owned(),
        capture_time: asset.date.time,
        import_time: shared.import_time,
        job_code: shared.job_code.map(|job_code| job_code.as_str().to_owned()),
        sequence_number: Some(sequence_number),
    };
    files
        .iter()
        .map(|file| PlannedFile {
            file_name: file.file_name.clone(),
            target: shared
                .template
                .render(&values, &file.file_name, shared.mode),
            date: asset.date.time,
            date_source: asset.date.source,
        })
        .collect()
}

fn file_times(file: &AssetFile) -> FileTimes {
    FileTimes {
        role: file.role,
        capture_time: file.capture_time,
        modification_time: file.modification_time,
    }
}

/// The base name of an asset: the stem of its first file that is not a sidecar, so
/// `DSC_0042.RAF.xmp` follows `DSC_0042.RAF`. An asset of only sidecars uses its
/// first file.
fn base_name(files: &[AssetFile]) -> Option<&str> {
    let named_by = files
        .iter()
        .find(|file| file.role != AssetFileRole::Sidecar)
        .or_else(|| files.first())?;
    Some(stem(&named_by.file_name))
}

/// The file name without its last extension.
fn stem(file_name: &str) -> &str {
    file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _extension)| stem)
}
