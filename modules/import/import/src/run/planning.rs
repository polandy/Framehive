// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Planning an import run: dating the selected files, looking them up in the import
//! history, and naming the files not imported before (ADR 0020 §1, IMPORT-002).

use domain::LocalDateTime;
use naming::{CounterDate, NameTemplate};

use super::recognition::Recognized;
use super::rename::{import_plan, rename};
use super::{FilePlan, ImportRun, ImportSettings, PlannedRun, RunFile};
use crate::plan::AssetFile;
use crate::report::{FileError, RunError};
use crate::scan::{FoundFile, LeftOut};
use crate::sequence::{Reservation, sequence_range};

/// What the import plan of a [`PlannedRun`] was made from, so that [`ImportRun::start`]
/// can name its files again with the reserved numbers.
#[derive(Debug)]
pub(super) struct PlanInputs {
    /// The dated files that are not imported before, in scan order; each is its own
    /// asset until grouping exists.
    pub(super) assets: Vec<AssetFile>,
    pub(super) import_time: LocalDateTime,
    pub(super) first_sequence_number: u32,
    /// Whether the numbers from `first_sequence_number` on are reserved.
    pub(super) reserved: bool,
}

impl ImportRun<'_> {
    /// Reserves the sequence numbers of `plan`, unless it is a dry run or they are
    /// reserved already, and names its files again if another import took the numbers
    /// the plan showed (IMPORT-002, "Reservation"). An import job reserves when it is
    /// enqueued (IMPORT-015, "Enqueueing"); [`ImportRun::start`] reserves otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`RunError::Sequence`] if the persisted counters cannot be read or
    /// stored, or the numbers do not fit the template; then nothing is reserved.
    pub(crate) fn reserve(
        &self,
        settings: &ImportSettings<'_>,
        mut plan: PlannedRun,
    ) -> Result<PlannedRun, RunError> {
        if settings.dry_run || plan.naming.reserved {
            return Ok(plan);
        }
        let first =
            self.first_sequence_number(settings.template, &plan.naming, Reservation::Reserve)?;
        if first != plan.naming.first_sequence_number {
            plan.naming.first_sequence_number = first;
            rename(&mut plan, settings);
        }
        plan.naming.reserved = true;
        Ok(plan)
    }

    /// The first sequence number of the import of `naming`, shown or reserved.
    pub(super) fn first_sequence_number(
        &self,
        template: &NameTemplate,
        naming: &PlanInputs,
        reservation: Reservation,
    ) -> Result<u32, RunError> {
        let count = u32::try_from(naming.assets.len()).unwrap_or(u32::MAX);
        let today = CounterDate::of(naming.import_time);
        let range = sequence_range(
            self.counters,
            self.session,
            template,
            today,
            count,
            reservation,
        )?;
        Ok(range.map_or(1, |range| range.first))
    }

    /// Dates `files`, looks them up in the import history and makes the import plan
    /// of the files not imported before at `import_time`, with the sequence numbers
    /// the import would get.
    pub(super) fn plan_at(
        &self,
        settings: &ImportSettings<'_>,
        files: Vec<FoundFile>,
        left_out: Vec<LeftOut>,
        import_time: LocalDateTime,
    ) -> Result<PlannedRun, RunError> {
        let mut checked: Vec<Result<Recognized, FileError>> = Vec::with_capacity(files.len());
        for file in &files {
            checked.push(match self.date(settings.source, file) {
                Ok(dated) => self.recognize(settings, file, dated)?,
                Err(error) => Err(error),
            });
        }
        let mut naming = PlanInputs {
            assets: checked
                .iter()
                .flatten()
                .filter(|file| file.imported_before.is_none())
                .map(|file| file.dated.asset_file.clone())
                .collect(),
            import_time,
            first_sequence_number: 1,
            reserved: false,
        };
        naming.first_sequence_number =
            self.first_sequence_number(settings.template, &naming, Reservation::Preview)?;
        let plan = import_plan(&naming, settings);
        // The plan has one entry for each file of `naming.assets`, in the same order.
        let mut planned = plan.files().iter();
        let files = files
            .into_iter()
            .zip(checked)
            .map(|(found, checked)| match checked {
                Ok(file) => RunFile {
                    found,
                    no_capture_time: file.dated.no_capture_time,
                    planned: match file.imported_before {
                        Some(target) => Ok(FilePlan::ImportedBefore { target }),
                        None => planned
                            .next()
                            .cloned()
                            .map(FilePlan::Named)
                            .ok_or(FileError::NotPlanned),
                    },
                    recognition: Some(file.recognition),
                },
                Err(error) => RunFile {
                    found,
                    no_capture_time: None,
                    planned: Err(error),
                    recognition: None,
                },
            })
            .collect();
        Ok(PlannedRun {
            free_space_unknown: Vec::new(),
            files,
            left_out,
            naming,
            recorded: None,
        })
    }
}
