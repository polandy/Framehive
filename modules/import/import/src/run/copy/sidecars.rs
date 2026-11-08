// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The XMP sidecars of each imported file: the one with the import's values and the
//! source sidecars that came along, written after the verified copy at the import
//! target and at each backup that got the file, read back and compared (IMPORT-019,
//! "Writing", IMPORT-020).

use std::io::{self, Read};
use std::path::{Path, PathBuf};

use domain::Checksum;
use files::ReadOnlyFileSystem;
use history::SidecarRecord;
use metadata::xmp::{
    FoundSidecar, SidecarError, SidecarErrorKind, SidecarFiles, SidecarStep, XmpPacket, parse_xmp,
    serialize_xmp,
};
use naming::DateSource;

use super::Imported;
use crate::backup::{BackupError, BackupOutcome};
use crate::conflicts::sidecar_target;
use crate::plan::PlannedFile;
use crate::report::{CarriedSidecar, ExistingValue, FileError, FileOutcome};
use crate::run::settings::PlannedSidecar;
use crate::run::{ImportRun, ImportSettings};
use crate::scan::FoundFile;

mod content;
mod kept;

use content::merge_values;

/// Writes the sidecars of one run: the job code and the `metadata` values of its
/// settings, with `{year}` of a file dated by its modification time taken from the
/// import date (IMPORT-019.6), and the source sidecars that came along (IMPORT-020).
pub(in crate::run) struct Sidecars<'s> {
    files: SidecarFiles<'s>,
    source_files: &'s dyn ReadOnlyFileSystem,
    settings: &'s ImportSettings<'s>,
    import_year: u16,
}

/// The content of one sidecar, made once for all its locations.
struct Prepared {
    content: Vec<u8>,
    /// The content, parsed: what each location must hold when read back; `None` for
    /// a source sidecar copied byte for byte.
    model: Option<XmpPacket>,
    checksum: Checksum,
    /// The values a source sidecar had that differ from the import's (IMPORT-020.7).
    existing_values: Vec<ExistingValue>,
}

/// Where the sidecars of one file go, and how.
struct Placement<'p> {
    target: &'p str,
    destination: &'p Path,
    year: u16,
    dry_run: bool,
    keep_existing: bool,
}

impl<'s> Sidecars<'s> {
    /// The sidecars of a run with `settings`, imported in `import_year`; `None` when
    /// the run has no XML syntax to write them with.
    pub(in crate::run) fn of_run(
        run: &'s ImportRun<'_>,
        settings: &'s ImportSettings<'s>,
        import_year: u16,
    ) -> Option<Self> {
        let files = SidecarFiles {
            files: run.file_system,
            hasher: run.hasher,
            xml: run.xml_syntax?,
            clock: run.clock,
        };
        Some(Sidecars {
            files,
            source_files: run.source_file_system,
            settings,
            import_year,
        })
    }

    /// Adds the sidecars of `file`, planned as `planned`, to what `imported` did: in a
    /// dry run only their names; otherwise writes them next to a copied file and to
    /// each backup that got the file. A file found already imported gets them only
    /// where none is there, so that a re-run completes a failed one (IMPORT-019.18);
    /// a source sidecar there counts where it holds what `history` recorded for it
    /// (IMPORT-020.9). A failure at the import target fails the file (IMPORT-019.16),
    /// one at a backup fails that backup (IMPORT-019.14).
    pub(super) fn add_to(
        &self,
        file: &FoundFile,
        planned: &PlannedFile,
        history: &[SidecarRecord],
        imported: &mut Imported,
    ) {
        let sidecars = self.settings.sidecars_of(file);
        let (target, dry_run, keep_existing) = match &imported.outcome {
            FileOutcome::Copied { target, .. } => (target.clone(), false, false),
            FileOutcome::WouldCopy { target } => (target.clone(), true, false),
            FileOutcome::AlreadyImported { target } => {
                (target.clone(), self.settings.dry_run, true)
            }
            _ => return,
        };
        let placement = Placement {
            target: &target,
            destination: self.settings.destination_of(file.role),
            year: self.year_of(planned),
            dry_run,
            keep_existing,
        };
        // The history keeps one sidecar per copy: the one with the values, else the first.
        let recorded = sidecars
            .iter()
            .position(|sidecar| sidecar.holds_values)
            .unwrap_or(0);
        imported.carried.clear();
        for (position, sidecar) in sidecars.iter().enumerate() {
            let carried =
                self.add_one(sidecar, &placement, position == recorded, history, imported);
            imported.carried.extend(carried);
        }
    }

    /// Adds one sidecar to what `imported` did; for a source sidecar, returns where it
    /// went and at how many locations it was written and verified.
    fn add_one(
        &self,
        sidecar: &PlannedSidecar<'_>,
        placement: &Placement<'_>,
        recorded: bool,
        history: &[SidecarRecord],
        imported: &mut Imported,
    ) -> Option<CarriedSidecar> {
        let name = sidecar_target(sidecar.naming, placement.target);
        let mut carried = sidecar.source.map(|source| CarriedSidecar {
            path: source.path.clone(),
            target: Some(name.clone()),
            verified_at: 0,
        });
        let at_import = placement.destination.join(&name);
        let expected = self.expected_where_kept(sidecar, placement, &name, history);
        let prepared = match self.needs_writing(&at_import, placement.keep_existing) {
            Ok(false) => return self.kept(carried, &expected, placement, &imported.backups),
            Ok(true) if placement.dry_run => {
                if sidecar.holds_values {
                    imported.sidecar = Some(name);
                }
                return carried;
            }
            Ok(true) => self.prepare(sidecar, &at_import, placement.year),
            Err(error) => Err(error),
        };
        let prepared = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                imported.outcome = FileOutcome::Failed(FileError::Sidecar(error));
                return carried;
            }
        };
        let mut verified_at = 0;
        match self.write_verified(&at_import, &prepared) {
            Ok(()) => {
                verified_at += 1;
                if sidecar.holds_values {
                    imported.sidecar = Some(name.clone());
                }
            }
            Err(error) => imported.outcome = FileOutcome::Failed(FileError::Sidecar(error)),
        }
        let kept = placement.keep_existing.then_some(expected.as_slice());
        let (at_backups, kept_at) = self.add_to_backups(&name, &prepared, kept, imported);
        if let Some(carried) = &mut carried {
            carried.verified_at = verified_at + at_backups;
        }
        if sidecar.holds_values {
            imported.existing_values = prepared.existing_values;
        }
        if recorded {
            imported.sidecar_kept_at = kept_at;
            imported.sidecar_record = Some(SidecarRecord {
                target: name,
                checksum: prepared.checksum,
            });
        }
        carried
    }

    /// The bytes of the sidecars of `file`, planned as `planned`: each source sidecar
    /// with the values packet added to the one that holds them, an upper bound of the
    /// merged size. A packet that cannot be serialized counts nothing; the run reports
    /// it when it writes it.
    pub(in crate::run) fn size_of(&self, file: &FoundFile, planned: &PlannedFile) -> u64 {
        self.settings
            .sidecars_of(file)
            .iter()
            .map(|sidecar| {
                let source = sidecar.source.map_or(0, |source| source.size);
                let values = if sidecar.holds_values {
                    self.values_size(self.year_of(planned)).unwrap_or(0)
                } else {
                    0
                };
                source.saturating_add(values)
            })
            .fold(0, u64::saturating_add)
    }

    fn values_size(&self, year: u16) -> Option<u64> {
        let mut packet = XmpPacket::new();
        merge_values(self.settings, year, &mut packet).ok()?;
        let content = serialize_xmp(&packet, self.files.clock, self.files.xml).ok()?;
        u64::try_from(content.len()).ok()
    }

    /// Writes `prepared` as `name` to each backup that got the file; with `kept`, the
    /// checksums a kept one counts with, only where none is there. Returns the number
    /// of backups it was written and verified at or kept with one of them, and the
    /// backups that keep their own.
    fn add_to_backups(
        &self,
        name: &str,
        prepared: &Prepared,
        kept: Option<&[Checksum]>,
        imported: &mut Imported,
    ) -> (usize, Vec<PathBuf>) {
        let mut verified_at = 0;
        let mut kept_at = Vec::new();
        for backup in &mut imported.backups {
            if !matches!(
                backup.outcome,
                BackupOutcome::BackedUp | BackupOutcome::AlreadyBackedUp
            ) {
                continue;
            }
            let at_backup = backup.destination.join(name);
            let written = self
                .needs_writing(&at_backup, kept.is_some())
                .and_then(|needed| {
                    if needed {
                        self.write_verified(&at_backup, prepared).map(|()| true)
                    } else {
                        Ok(false)
                    }
                });
            match written {
                Ok(true) => verified_at += 1,
                Ok(false) => {
                    verified_at += usize::from(self.holds(&at_backup, kept.unwrap_or_default()));
                    kept_at.push(backup.destination.clone());
                }
                Err(error) => {
                    backup.outcome = BackupOutcome::Failed(BackupError::Sidecar(error));
                }
            }
        }
        (verified_at, kept_at)
    }

    /// The year `{year}` stands for: the capture year, or the year of the import date
    /// when the file is dated by its modification time (IMPORT-019.6).
    fn year_of(&self, planned: &PlannedFile) -> u16 {
        match planned.date_source {
            DateSource::CaptureTime => planned.date.year(),
            DateSource::ModificationTime => self.import_year,
        }
    }

    /// The content of `sidecar` at `path`: the values packet of a file of `year`,
    /// merged into the source sidecar when one came along, serialized and parsed
    /// again; a source sidecar without values as it is (IMPORT-020.5).
    fn prepare(
        &self,
        sidecar: &PlannedSidecar<'_>,
        path: &Path,
        year: u16,
    ) -> Result<Prepared, SidecarError> {
        let source = match sidecar.source {
            Some(source) => Some(self.read_source(&self.settings.source.join(&source.path))?),
            None => None,
        };
        let (content, model, existing_values) = match source {
            Some((_, content)) if !sidecar.holds_values => (content, None, Vec::new()),
            source => {
                let (packet, existing_values) = self.merged(source, path, year)?;
                let content = serialize_xmp(&packet, self.files.clock, self.files.xml)
                    .map_err(|error| sidecar_error(path, SidecarErrorKind::Clock(error)))?
                    .into_bytes();
                let model = parse_xmp(path, &content, self.files.xml).map_err(|error| {
                    sidecar_error(path, SidecarErrorKind::Unreadable(error.reason))
                })?;
                (content, Some(model), existing_values)
            }
        };
        let mut hash = self.files.hasher.start();
        hash.update(&content);
        Ok(Prepared {
            content,
            model,
            checksum: hash.finish(),
            existing_values,
        })
    }

    /// The values of a file of `year` merged into the parsed `source`, or into an
    /// empty packet; `path` names the target in an error.
    fn merged(
        &self,
        source: Option<(PathBuf, Vec<u8>)>,
        path: &Path,
        year: u16,
    ) -> Result<(XmpPacket, Vec<ExistingValue>), SidecarError> {
        let mut packet = match source {
            Some((source_path, content)) => parse_xmp(&source_path, &content, self.files.xml)
                .map_err(|error| {
                    sidecar_error(&source_path, SidecarErrorKind::Unreadable(error.reason))
                })?,
            None => XmpPacket::new(),
        };
        let existing = merge_values(self.settings, year, &mut packet)
            .map_err(|error| sidecar_error(path, SidecarErrorKind::Edit(error)))?;
        Ok((packet, existing))
    }

    /// The source sidecar at `path`, with its content.
    fn read_source(&self, path: &Path) -> Result<(PathBuf, Vec<u8>), SidecarError> {
        let mut content = Vec::new();
        self.source_files
            .open_read(path)
            .and_then(|mut reader| reader.read_to_end(&mut content))
            .map_err(|source| {
                let step = SidecarStep::Read;
                sidecar_error(path, SidecarErrorKind::Io { step, source })
            })?;
        Ok((path.to_path_buf(), content))
    }

    /// Whether the sidecar at `path` is to be written: always, unless `keep_existing`
    /// and something is there already.
    fn needs_writing(&self, path: &Path, keep_existing: bool) -> Result<bool, SidecarError> {
        if !keep_existing {
            return Ok(true);
        }
        let entry = self.files.files.entry(path).map_err(|source| {
            let step = SidecarStep::Read;
            sidecar_error(path, SidecarErrorKind::Io { step, source })
        })?;
        Ok(entry.is_none())
    }

    /// Writes `prepared` new at `path`, through a `.part` file and a rename, and
    /// requires it to read back byte for byte and to parse to the same packet.
    fn write_verified(&self, path: &Path, prepared: &Prepared) -> Result<(), SidecarError> {
        let content = prepared.content.as_slice();
        self.files.write(path, content, &FoundSidecar::nothing())?;
        let back = self.read_back(path).map_err(|source| {
            let step = SidecarStep::Read;
            sidecar_error(path, SidecarErrorKind::Io { step, source })
        })?;
        if back != content {
            return Err(sidecar_error(path, SidecarErrorKind::Differs));
        }
        let Some(model) = &prepared.model else {
            return Ok(());
        };
        let parsed = parse_xmp(path, &back, self.files.xml)
            .map_err(|error| sidecar_error(path, SidecarErrorKind::Unreadable(error.reason)))?;
        if parsed != *model {
            return Err(sidecar_error(path, SidecarErrorKind::Differs));
        }
        Ok(())
    }

    fn read_back(&self, path: &Path) -> io::Result<Vec<u8>> {
        let mut content = Vec::new();
        self.files
            .files
            .open_read(path)?
            .read_to_end(&mut content)?;
        Ok(content)
    }
}

fn sidecar_error(path: &Path, kind: SidecarErrorKind) -> SidecarError {
    SidecarError {
        path: path.to_path_buf(),
        kind,
        part_left: None,
    }
}
