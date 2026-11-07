# 10. Glossary (Ubiquitous Language)

Part of the [architecture](README.md).

Names in the code follow this glossary.

| Term | Code name | Meaning |
|---|---|---|
| Asset | `Asset` | An image or video together with all its associated files (RAW, JPG, sidecars) |
| Asset file | `AssetFile` | One file of an asset, with its role: `Raw`, `Rendered` (camera JPEG/HEIF), `Video`, `Sidecar` |
| Grouping | `grouping` | Rule that derives assets from files: same folder, same base name, different extension (LIBRARY-001) |
| Stack | `Stack` | Several assets shown as one tile: burst, exposure bracketing, versions. Not to be confused with an asset |
| Sidecar | `Sidecar` | XMP file next to the original, carrier of the metadata. As an `AssetFile` role it also covers a camera's video thumbnail file (`.THM`) |
| Library | `Library` | The totality of all storage locations that Framehive manages |
| Library root | `LibraryRoot` | A root directory with a marker file and a UUID |
| Volume | `Volume` | Physical or network storage medium on which library roots reside |
| Source | `Source` | What is imported from: camera, card, phone, folder |
| Device | `Device` | A memory card mounted as a file system, identified by its mount point, or a camera connected over PTP or MTP, identified by its model and port, as it is offered for import |
| Device source | `DeviceSource` | Port of the `devices` service that lists the cards that are present, sorted by mount point, and ejects one; implemented by `adapter-mounts` (DEVICES-001) |
| Device watch | `DeviceWatch` | Polls the device source and the cameras, compares the device list with that of the last poll and reports removed, then added devices; a kind that cannot be listed keeps its last devices (DEVICES-001) |
| Guided import | `framehive` without arguments | The line-based dialog that waits for a memory card, shows the plan, imports the card on Enter and ejects it; it remembers its answers (CLI-002, CLI-003, ADR 0024) |
| Name template | `NameTemplate` | Text that derives an imported file's folders and name (IMPORT-001) |
| Job code | `JobCode` | A short label the user gives an import, inserted by the `{job}` token of a name template (IMPORT-006). Not to be confused with a job |
| Recent job codes | `RecentJobCodes` | The job codes used before, most recently used first, each at most once, at most 10 (IMPORT-006) |
| Import profile | `ImportProfile` | A named set of import settings in a YAML file, one file per profile, named after it: destination, template, conflict policy, job codes, backup destinations (IMPORT-011, ADR 0032) |
| Configuration folder | `config_folder` | The folder of the files a user edits: `$XDG_CONFIG_HOME/framehive` on Linux, `~/Library/Application Support/framehive` on macOS (CLI-010, ADR 0032). Not to be confused with the state folder of ADR 0025 |
| Capture time | `CaptureTime` | The wall-clock time the camera recorded, from EXIF `DateTimeOriginal`, with optional sub-seconds and UTC offset (CORE-005) |
| Full decode | `PreviewDecoder::decode_full`, `DecodedImage` | Decoding a RAW at full resolution into 16-bit sRGB samples, for 100% zoom and comparison (PREVIEWS-001) |
| Reference decode | `DecodeMode::Reference` | Full decode with one thread, bit-identical between runs; its pixel hashes pin the decoder version (PREVIEWS-001, ADR 0028) |
| Timestamp | `Timestamp` | An instant in UTC, to the nanosecond, without calendar or time zone (ADR 0022) |
| UTC offset | `UtcOffset` | The difference between local time and UTC at an instant, in seconds east of UTC |
| Clock | `Clock` | Port in `domain` for the current timestamp and the local UTC offset at a timestamp; implemented by `adapter-clock` |
| Local date-time | `LocalDateTime` | A calendar date and wall-clock time without a time zone, to the second; capture times and names use it |
| Conflict policy | `ConflictPolicy` | What the import does when a target name holds a different file: `add_suffix`, `skip` or `fail` (IMPORT-003) |
| Import plan | `ImportPlan` | A pure description: which file goes where, with what name, and which conflicts |
| Planned run | `PlannedRun` | An import run that is planned and not yet executed: the selected files of a source scan, each dated and named by one import plan; it writes nothing |
| Import run | `ImportRun` | The execution of an import plan: copies each file and reports its progress and result (ADR 0020) |
| Import history | `ImportHistory` | The machine-local record of every verified copy of an import run, by which a later run recognizes a file imported before by its content (IMPORT-009, ADR 0031) |
| Import log | `LoggedRun`, `RunLog` | The view of the import history that lists the import runs and shows, for one run, which file of the source went where with its checksum; it never writes the history (IMPORT-010, CLI-009) |
| Sequence counter | `SequenceCounters` | The stored values of the persisted sequence kinds (`imported_today`, `stored`), from which an import reserves a contiguous range of numbers when its import job is enqueued, or when an import run starts (IMPORT-002, ADR 0029, ADR 0036) |
| Video destination | `video_destination` | The folder that receives the videos of an import run instead of the import destination, with video backup destinations of their own; without one, videos go where photos go (IMPORT-013) |
| Backup destination | `BackupDestination` | A folder that receives each imported file as well, at the same target path as the import destination, verified against the source checksum and never overwritten (IMPORT-007, ADR 0030) |
| Format readiness | `FormatReadiness` | Whether a finished import run verified every file of the source at the import destination and at each backup destination, so the card can be formatted, and which files keep it from being formatted (IMPORT-008) |
| Free space | `FreeSpace` | The bytes an unprivileged process may still write on the file system of a folder, with that file system's identity; the import checks it per file system before the first copy (IMPORT-012, ADR 0033) |
| Verified copy | `copy_verified` | A copy through a `.part` file whose content is checked against the source checksum before it counts as copied (IMPORT-004) |
| Checksum | `Checksum` | Hash of a file's content with its algorithm, written `blake3:<hex>` (architecture §8) |
| File system | `FileSystem` | Port of the `files` service for files and folders on a storage device, with explicit syncing; implemented by `adapter-fs` |
| Read-only file system | `ReadOnlyFileSystem` | The reading part of `FileSystem`: entries, folder lists, sizes, modification times and reading from any offset; the import reads its source only through it (ADR 0034) |
| Camera | `Camera` | A camera or phone connected over PTP or MTP, with its model and its port, for example `usb:001,005`; opened, it is a `ReadOnlyFileSystem` with one folder per storage (IMPORT-014) |
| Cameras | `Cameras` | Port of the `files` service that lists the connected cameras and opens one; implemented by `adapter-gphoto2` with libgphoto2 (ADR 0034) |
| Hasher | `Hasher` | Port of the `files` service that computes a checksum; implemented by `adapter-blake3` |
| Part file | `.part` file | `<target>.part` next to the target path; a verified copy writes it and renames it to the target only when it is complete (IMPORT-004) |
| Job | `Job` | Background work in the job queue; it runs on a worker thread of its own and ends `Finished`, `Stopped` or `Failed` (CORE-007, ADR 0035) |
| Job queue | `JobQueue` | Runs jobs in parallel unless they conflict, conflicting ones first in, first out; stops a running job, removes a queued one, and reports each job's state over a channel of its own (CORE-007, ADR 0035, ADR 0036) |
| Resource key | `ResourceKey` | An opaque name of something a job uses, such as a source device or a destination file system; two jobs that name a common resource key conflict and run one after another (CORE-007, ADR 0036) |
| Job ID | `JobId` | Names a job in its job queue; numbered from 1 per queue in the order the jobs were enqueued (CORE-007) |
| Job state | `JobState` | Where a job is: `Queued`, `Running`, `Finished`, `Stopped`, `Failed` with a message, or `Removed` (CORE-007) |
| Dry-run memory | `DryRunMemory` | The sequence counters and target names that the dry-run import jobs of one command reserve in memory, so a dry run of several sources shows what a real run would do; the persisted counters are never written (IMPORT-016) |
| Import job | `ImportJob` | An import run as a job: it owns its settings, ports and source, reports each step to whoever enqueued it, and hands an opened camera back when it ends; enqueued with its plan, it reserves its sequence range and target names (IMPORT-015, ADR 0036) |
| Reserved target names | `ReservedTargetNames` | The target names that the import jobs of one application reserved when they were enqueued; a reserved name counts as taken for the conflicts of the other jobs (IMPORT-015, ADR 0036) |
| Index | `Index` | Machine-local database, reconstructible from files |
| Rating | `Rating` | 0–5 stars |
| Pick state | `PickState` | Keep / pending / rejected |
| Color label | `ColorLabel` | Color used as a marker |
| Tag | `Tag` | Hierarchical keyword, path with `/` |
| Region | `Region` | Image area, e.g. a face, per MWG |
| Module | `Module` | Business unit as defined in Section 4 |
