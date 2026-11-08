// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A scripted terminal for the golden tests: memory file system, memory device source,
//! a manual clock and a list of input steps.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io::{self, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use adapter_xmp::QuickXml;
use devices::testing::MemoryDeviceSource;
use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::testing::{FnvHasher, MemoryCameras, MemoryFileSystem, MemoryFreeSpace};
use files::{
    EntryKind, FileSystem, FolderEntry, FreeSpace, ReadOnlyFileSystem, ReadSeek, WriteFile,
};
use import::{ImportMetadata, ImportPorts, RecentJobCodes, ReservedTargetNames, SessionSequence};
use naming::NameTemplate;
use naming::testing::MemorySequenceCounters;

use crate::guided::dialog::Dialog;
use crate::guided::events::{Event, EventSource};
use crate::guided::state::{RememberedState, load_state, save_state, state_file};
use history::ImportHistory;
use history::testing::MemoryImportHistory;

use super::shared::{HarnessCameras, Shared};
use crate::guided::{Environment, Ports};
use crate::paths::PathError;

/// The home folder of the tests.
pub(super) const HOME: &str = "/home/ada";
/// The default destination for that home folder.
pub(super) const PICTURES: &str = "/home/ada/Pictures";
/// The folder cards are mounted in.
pub(super) const MEDIA: &str = "/run/media/ada";
/// 2026-10-17 14:05:09 UTC.
pub(super) const OCTOBER_17: i64 = 1_792_245_909;
/// 2026-10-19 10:00:00 UTC.
pub(super) const OCTOBER_19: i64 = 1_792_404_000;
/// The preset templates, by number.
pub(super) const PRESET_1: &str = "{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}";
pub(super) const PRESET_2: &str = "{date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}";
pub(super) const PRESET_3: &str = "{date:%Y}/{date:%Y-%m}/{date:%Y%m%d}_{number:4}.{ext}";

/// One step of the person at the terminal.
pub(super) enum Step {
    /// Types a line and Enter.
    Line(&'static str),
    /// Waits a poll interval without typing.
    Tick,
    /// Presses Ctrl+D.
    End,
    /// Inserts the card with this name.
    Insert(&'static str),
    /// Pulls the card with this name.
    Remove(&'static str),
    /// Changes the world, for example connects or disconnects a camera.
    Call(fn(&Harness)),
}

/// What the dialog wrote.
pub(super) struct Transcript {
    /// Standard output, with the typed lines echoed as a terminal shows them.
    pub(super) output: String,
    /// Standard error.
    pub(super) warnings: String,
}

/// The mount point of the card `name`.
pub(super) fn mount(name: &str) -> PathBuf {
    Path::new(MEDIA).join(name)
}

/// A change of the world after this many copies into the library.
pub(super) type AfterCopies = (usize, fn(&Harness));

/// The world of one golden test. The import job reads and writes it on the worker
/// thread while the dialog waits for its answer (IMPORT-015), so it is shared and
/// every part that changes is behind a lock.
#[derive(Clone)]
pub(super) struct Harness {
    world: Arc<World>,
    pub(super) counters: Arc<MemorySequenceCounters>,
    pub(super) session: Arc<SessionSequence>,
    /// The import history, kept across the runs of this harness as the history in the
    /// state folder is kept across program runs.
    pub(super) history: Arc<dyn ImportHistory + Send + Sync>,
    /// The free space of the destinations; unlimited unless a test sets it.
    pub(super) free_space: Arc<dyn FreeSpace + Send + Sync>,
    /// Resolves a folder to its canonical path; the paths of the memory file system are
    /// canonical already, so it returns them unchanged unless a test replaces it.
    pub(super) resolve_folder: fn(&Path) -> Result<PathBuf, PathError>,
}

/// The part of the harness that the dialog and the import job share.
pub(super) struct World {
    pub(super) memory: MemoryFileSystem,
    pub(super) clock: Arc<ManualClock>,
    pub(super) devices: MemoryDeviceSource,
    /// The connected cameras; steps replace them as cameras come and go.
    pub(super) cameras: Shared<MemoryCameras>,
    /// The storage of the test camera, shared with the cameras it is connected as.
    pub(super) camera_storage: MemoryFileSystem,
    /// After this many copies into the library, this happens, for example a camera is
    /// disconnected.
    pub(super) after_copies: Shared<Option<AfterCopies>>,
    /// Each copy into the library advances the clock this much.
    pub(super) time_per_copy: Shared<Duration>,
    /// After this many copies, the next read of a card pulls it and fails.
    pub(super) pull_after_copies: Shared<Option<usize>>,
    copies: Shared<usize>,
    /// The card that was pulled during a read, unreadable until inserted again.
    pulled: Shared<Option<PathBuf>>,
    /// Sizes that the file system reports instead of the real ones.
    sizes: Shared<HashMap<PathBuf, u64>>,
}

impl Deref for Harness {
    type Target = World;

    fn deref(&self) -> &World {
        &self.world
    }
}

impl Harness {
    pub(super) fn new() -> Self {
        let memory = MemoryFileSystem::new();
        memory.add_folder(Path::new(HOME));
        let world = World {
            memory,
            clock: Arc::new(ManualClock::new(
                Timestamp::from_unix_seconds(OCTOBER_19 + 86_400),
                UtcOffset::UTC,
            )),
            devices: MemoryDeviceSource::new(),
            cameras: Shared::new(MemoryCameras::new()),
            camera_storage: MemoryFileSystem::new(),
            after_copies: Shared::new(None),
            time_per_copy: Shared::new(Duration::ZERO),
            pull_after_copies: Shared::new(None),
            copies: Shared::new(0),
            pulled: Shared::new(None),
            sizes: Shared::new(HashMap::new()),
        };
        Harness {
            world: Arc::new(world),
            counters: Arc::new(MemorySequenceCounters::new()),
            session: Arc::new(SessionSequence::new()),
            history: Arc::new(MemoryImportHistory::new()),
            free_space: Arc::new(MemoryFreeSpace::unlimited()),
            resolve_folder: |folder| Ok(folder.to_path_buf()),
        }
    }

    /// The state file for the home folder, without `XDG_STATE_HOME`.
    pub(super) fn state_file() -> PathBuf {
        state_file(None, Path::new(HOME))
    }

    /// Adds a file to the card `card` at `path` below its mount point.
    pub(super) fn add_file(&self, card: &str, path: &str, content: &[u8], unix_seconds: i64) {
        let modified = Timestamp::from_unix_seconds(unix_seconds);
        self.memory
            .add_file(&mount(card).join(path), content, modified);
    }

    /// Makes the file system report `size` for the file at `path` on `card`.
    pub(super) fn pretend_size(&self, card: &str, path: &str, size: u64) {
        self.sizes.borrow_mut().insert(mount(card).join(path), size);
    }

    /// Inserts the card `name` before the dialog starts.
    pub(super) fn insert(&self, name: &str) {
        self.memory.add_folder(&mount(name).join("DCIM"));
        self.devices.insert_card(mount(name));
    }

    /// Writes a state file with these answers.
    pub(super) fn remember(
        &self,
        destination: Option<&str>,
        template: Option<&str>,
        job_codes: &[&str],
    ) {
        let state = RememberedState {
            destination: destination.map(PathBuf::from),
            template: template.map(|text| NameTemplate::parse(text).unwrap()),
            job_codes: RecentJobCodes::from_stored(job_codes),
            ..RememberedState::default()
        };
        save_state(&self.memory, &Self::state_file(), &state).unwrap();
    }

    /// Adds the backup destinations `backups` to the remembered answers (CLI-006).
    pub(super) fn remember_backups(&self, backups: &[&str]) {
        let mut state = load_state(&self.memory, &Self::state_file()).state;
        state.backups = backups.iter().map(PathBuf::from).collect();
        save_state(&self.memory, &Self::state_file(), &state).unwrap();
    }

    /// The remembered answers as the state file holds them.
    pub(super) fn state_text(&self) -> String {
        String::from_utf8(self.memory.content(&Self::state_file()).unwrap_or_default()).unwrap()
    }

    /// Runs the dialog through `steps`; once they are used up, the input ends.
    pub(super) fn run(&self, steps: Vec<Step>) -> Transcript {
        let import = ImportPorts {
            file_system: Arc::new(HookedFileSystem {
                harness: self.clone(),
            }),
            hasher: Arc::new(FnvHasher),
            clock: Arc::clone(&self.clock) as _,
            counters: Arc::clone(&self.counters) as _,
            session: Arc::clone(&self.session),
            history: Arc::clone(&self.history),
            free_space: Arc::clone(&self.free_space),
            reserved_names: Arc::new(ReservedTargetNames::new()),
            xml_syntax: Arc::new(QuickXml::new()),
        };
        let ports = Ports {
            import: &import,
            devices: &self.devices,
            cameras: &HarnessCameras { harness: self },
            resolve_folder: self.resolve_folder,
        };
        let environment = Environment {
            home: PathBuf::from(HOME),
            current_folder: PathBuf::from("/tmp/work"),
            state_file: Self::state_file(),
            state_folder: PathBuf::from(HOME).join(".local/state/framehive"),
            metadata: ImportMetadata::default(),
        };
        let screen = Rc::new(RefCell::new(Vec::new()));
        let mut events = ScriptedEvents {
            harness: self,
            steps: steps.into(),
            screen: Rc::clone(&screen),
        };
        let mut output = Screen(Rc::clone(&screen));
        let mut warnings = Vec::new();
        Dialog::start(ports, environment, &mut events, &mut output, &mut warnings).run();
        let output = String::from_utf8(screen.borrow().clone()).unwrap();
        Transcript {
            output,
            warnings: String::from_utf8(warnings).unwrap(),
        }
    }
}

/// The terminal screen, shared by the dialog's output and the echo of typed lines.
struct Screen(Rc<RefCell<Vec<u8>>>);

impl Write for Screen {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct ScriptedEvents<'h> {
    harness: &'h Harness,
    steps: VecDeque<Step>,
    screen: Rc<RefCell<Vec<u8>>>,
}

impl EventSource for ScriptedEvents<'_> {
    fn next_event(&mut self) -> Event {
        while let Some(step) = self.steps.pop_front() {
            match step {
                Step::Line(line) => {
                    self.screen
                        .borrow_mut()
                        .extend_from_slice(format!("{line}\n").as_bytes());
                    return Event::Line(line.to_owned());
                }
                Step::Tick => return Event::Tick,
                Step::End => return Event::End,
                Step::Insert(name) => {
                    self.harness.pulled.borrow_mut().take();
                    self.harness.devices.insert_card(mount(name));
                }
                Step::Remove(name) => self.harness.devices.remove(&mount(name)),
                Step::Call(change) => change(self.harness),
            }
        }
        Event::End
    }
}

/// The memory file system with the test's hooks: the clock advances per copy, a card
/// can be pulled during a read, and sizes can be pretended.
struct HookedFileSystem {
    harness: Harness,
}

impl HookedFileSystem {
    fn card_of(path: &Path) -> Option<PathBuf> {
        let rest = path.strip_prefix(MEDIA).ok()?;
        let name = rest.components().next()?;
        Some(Path::new(MEDIA).join(name))
    }

    /// Fails reads of a pulled card, and pulls the card at the read after the set number
    /// of copies.
    fn check_read(&self, path: &Path) -> io::Result<()> {
        let Some(card) = Self::card_of(path) else {
            return Ok(());
        };
        let harness = &self.harness;
        let due = harness.pull_after_copies.get() == Some(harness.copies.get());
        if due && harness.pulled.borrow().is_none() {
            harness.pull_after_copies.set(None);
            harness.devices.remove(&card);
            *harness.pulled.borrow_mut() = Some(card.clone());
        }
        if harness.pulled.borrow().as_deref() == Some(card.as_path()) {
            harness.clock.advance(harness.time_per_copy.get());
            return Err(io::Error::other("the card was pulled"));
        }
        Ok(())
    }
}

impl ReadOnlyFileSystem for HookedFileSystem {
    fn entry(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        self.harness.memory.entry(path)
    }

    fn list_folder(&self, path: &Path) -> io::Result<Vec<FolderEntry>> {
        self.harness.memory.list_folder(path)
    }

    fn file_size(&self, path: &Path) -> io::Result<u64> {
        match self.harness.sizes.borrow().get(path) {
            Some(size) => Ok(*size),
            None => self.harness.memory.file_size(path),
        }
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        self.check_read(path)?;
        self.harness.memory.open_read(path)
    }

    fn modification_time(&self, path: &Path) -> io::Result<Timestamp> {
        self.harness.memory.modification_time(path)
    }
}

impl FileSystem for HookedFileSystem {
    fn create_folder(&self, path: &Path) -> io::Result<()> {
        self.harness.memory.create_folder(path)
    }

    fn create_file(&self, path: &Path) -> io::Result<Box<dyn WriteFile>> {
        self.harness.memory.create_file(path)
    }

    fn replace_file(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.harness.memory.replace_file(from, to)
    }

    fn rename_no_replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.harness.memory.rename_no_replace(from, to)?;
        if to.starts_with(PICTURES) || to.starts_with("/home/ada/Archive") {
            let copies = self.harness.copies.get() + 1;
            self.harness.copies.set(copies);
            self.harness.clock.advance(self.harness.time_per_copy.get());
            if let Some((after, change)) = self.harness.after_copies.get()
                && after == copies
            {
                self.harness.after_copies.set(None);
                change(&self.harness);
            }
        }
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.harness.memory.remove_file(path)
    }

    fn sync_folder(&self, path: &Path) -> io::Result<()> {
        self.harness.memory.sync_folder(path)
    }
}
