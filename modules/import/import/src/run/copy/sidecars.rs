// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The XMP sidecar of each imported photo: written after the verified copy at the
//! import target and at each backup that got the file, read back and compared
//! (IMPORT-019, "Writing").

use std::io::{self, Read};
use std::path::Path;

use domain::{AssetFileRole, Checksum};
use history::SidecarRecord;
use metadata::xmp::namespaces::{DUBLIN_CORE, PHOTOSHOP};
use metadata::xmp::{
    FoundSidecar, SidecarError, SidecarErrorKind, SidecarFiles, SidecarStep, XmpName, XmpPacket,
    parse_xmp, serialize_xmp,
};
use naming::DateSource;

use super::Imported;
use crate::backup::{BackupError, BackupOutcome};
use crate::conflicts::sidecar_target;
use crate::metadata_values::Keyword;
use crate::plan::PlannedFile;
use crate::report::{FileError, FileOutcome};
use crate::run::{ImportRun, ImportSettings};

/// Writes the sidecars of one run: the job code and the `metadata` values of its
/// settings, with `{year}` of a file dated by its modification time taken from the
/// import date (IMPORT-019.6).
pub(in crate::run) struct Sidecars<'s> {
    files: SidecarFiles<'s>,
    settings: &'s ImportSettings<'s>,
    import_year: u16,
}

/// The content of one file's sidecar, serialized once for all its locations.
struct Prepared {
    content: String,
    /// The content, parsed: what each location must hold when read back.
    model: XmpPacket,
    checksum: Checksum,
}

impl<'s> Sidecars<'s> {
    /// The sidecars of a run with `settings`, imported in `import_year`; `None` when
    /// the run writes none or has no XML syntax to write them with.
    pub(in crate::run) fn of_run(
        run: &'s ImportRun<'_>,
        settings: &'s ImportSettings<'s>,
        import_year: u16,
    ) -> Option<Self> {
        let xml = run.xml_syntax.filter(|_| settings.writes_sidecars())?;
        let files = SidecarFiles {
            files: run.file_system,
            hasher: run.hasher,
            xml,
            clock: run.clock,
        };
        Some(Sidecars {
            files,
            settings,
            import_year,
        })
    }

    /// Adds the sidecar of a file of `role`, planned as `planned`, to what `imported`
    /// did: in a dry run only its name; otherwise writes it next to a copied file and
    /// to each backup that got the file. A file found already imported gets it only
    /// where none is there, so that a re-run completes a failed one (IMPORT-019.18).
    /// A failure at the import target fails the file (IMPORT-019.16), one at a backup
    /// fails that backup (IMPORT-019.14).
    pub(super) fn add_to(
        &self,
        role: AssetFileRole,
        planned: &PlannedFile,
        imported: &mut Imported,
    ) {
        let Some(naming) = self.settings.sidecar_naming_of(role) else {
            return;
        };
        let (target, dry_run, keep_existing) = match &imported.outcome {
            FileOutcome::Copied { target, .. } => (target, false, false),
            FileOutcome::WouldCopy { target } => (target, true, false),
            FileOutcome::AlreadyImported { target } => (target, self.settings.dry_run, true),
            _ => return,
        };
        let name = sidecar_target(naming, target);
        let at_import = self.settings.destination_of(role).join(&name);
        match self.needs_writing(&at_import, keep_existing) {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                imported.outcome = FileOutcome::Failed(FileError::Sidecar(error));
                return;
            }
        }
        if dry_run {
            imported.sidecar = Some(name);
            return;
        }
        let prepared = match self.prepare(&at_import, self.year_of(planned)) {
            Ok(prepared) => prepared,
            Err(error) => {
                imported.outcome = FileOutcome::Failed(FileError::Sidecar(error));
                return;
            }
        };
        match self.write_verified(&at_import, &prepared) {
            Ok(()) => imported.sidecar = Some(name.clone()),
            Err(error) => imported.outcome = FileOutcome::Failed(FileError::Sidecar(error)),
        }
        for backup in &mut imported.backups {
            if !matches!(
                backup.outcome,
                BackupOutcome::BackedUp | BackupOutcome::AlreadyBackedUp
            ) {
                continue;
            }
            let at_backup = backup.destination.join(&name);
            let written = self
                .needs_writing(&at_backup, keep_existing)
                .and_then(|needed| {
                    if needed {
                        self.write_verified(&at_backup, &prepared)
                    } else {
                        Ok(())
                    }
                });
            if let Err(error) = written {
                backup.outcome = BackupOutcome::Failed(BackupError::Sidecar(error));
            }
        }
        imported.sidecar_record = Some(SidecarRecord {
            target: name,
            checksum: prepared.checksum,
        });
    }

    /// The year `{year}` stands for: the capture year, or the year of the import date
    /// when the file is dated by its modification time (IMPORT-019.6).
    fn year_of(&self, planned: &PlannedFile) -> u16 {
        match planned.date_source {
            DateSource::CaptureTime => planned.date.year(),
            DateSource::ModificationTime => self.import_year,
        }
    }

    /// The packet of a file of `year`, serialized and parsed again; `path` names the
    /// sidecar in an error.
    fn prepare(&self, path: &Path, year: u16) -> Result<Prepared, SidecarError> {
        let fail = |kind| sidecar_error(path, kind);
        let packet = self
            .packet(year)
            .map_err(|error| fail(SidecarErrorKind::Edit(error)))?;
        let content = serialize_xmp(&packet, self.files.clock, self.files.xml)
            .map_err(|error| fail(SidecarErrorKind::Clock(error)))?;
        let model = parse_xmp(path, content.as_bytes(), self.files.xml)
            .map_err(|error| fail(SidecarErrorKind::Unreadable(error.reason)))?;
        let mut hash = self.files.hasher.start();
        hash.update(content.as_bytes());
        Ok(Prepared {
            content,
            model,
            checksum: hash.finish(),
        })
    }

    /// The job code as `photoshop:TransmissionReference`, the creators as the
    /// `dc:creator` Seq, the copyright as `dc:rights` x-default and the keywords
    /// (ADR 0040).
    fn packet(&self, year: u16) -> Result<XmpPacket, metadata::xmp::XmpEditError> {
        let metadata = self.settings.metadata;
        let mut packet = XmpPacket::new();
        if let Some(job_code) = self.settings.job_code {
            let name = XmpName::new(PHOTOSHOP, "TransmissionReference");
            packet.set_text(&name, job_code.as_str())?;
        }
        if !metadata.creator.is_empty() {
            let creators: Vec<&str> = metadata.creator.iter().map(String::as_str).collect();
            packet.replace_seq(&XmpName::new(DUBLIN_CORE, "creator"), &creators)?;
        }
        if let Some(rights) = metadata.copyright_of(i32::from(year)) {
            packet.set_language(&XmpName::new(DUBLIN_CORE, "rights"), "x-default", &rights)?;
        }
        let keywords: Vec<&[String]> = metadata.keywords.iter().map(Keyword::levels).collect();
        if !keywords.is_empty() {
            packet.add_keywords(&keywords)?;
        }
        Ok(packet)
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
        let content = prepared.content.as_bytes();
        self.files.write(path, content, &FoundSidecar::nothing())?;
        let back = self.read_back(path).map_err(|source| {
            let step = SidecarStep::Read;
            sidecar_error(path, SidecarErrorKind::Io { step, source })
        })?;
        let parsed = parse_xmp(path, &back, self.files.xml)
            .map_err(|error| sidecar_error(path, SidecarErrorKind::Unreadable(error.reason)))?;
        if back != content || parsed != prepared.model {
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
