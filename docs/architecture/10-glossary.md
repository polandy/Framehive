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
| Name template | `NameTemplate` | Text that derives an imported file's folders and name (IMPORT-001) |
| Capture time | `CaptureTime` | The wall-clock time the camera recorded, from EXIF `DateTimeOriginal`, with optional sub-seconds and UTC offset (CORE-005) |
| Timestamp | `Timestamp` | An instant in UTC, to the nanosecond, without calendar or time zone (ADR 0022) |
| UTC offset | `UtcOffset` | The difference between local time and UTC at an instant, in seconds east of UTC |
| Clock | `Clock` | Port in `domain` for the current timestamp and the local UTC offset at a timestamp; implemented by `adapter-clock` |
| Local date-time | `LocalDateTime` | A calendar date and wall-clock time without a time zone, to the second; capture times and names use it |
| Conflict policy | `ConflictPolicy` | What the import does when a target name holds a different file: `add_suffix`, `skip` or `fail` (IMPORT-003) |
| Import plan | `ImportPlan` | A pure description: which file goes where, with what name, and which conflicts |
| Import run | `ImportRun` | The execution of an import plan: copies each file and reports its progress and result (ADR 0020) |
| Verified copy | `copy_verified` | A copy through a `.part` file whose content is checked against the source checksum before it counts as copied (IMPORT-004) |
| Checksum | `Checksum` | Hash of a file's content with its algorithm, written `blake3:<hex>` (architecture §8) |
| File system | `FileSystem` | Port of the `files` service for files and folders on a storage device, with explicit syncing; implemented by `adapter-fs` |
| Hasher | `Hasher` | Port of the `files` service that computes a checksum; implemented by `adapter-blake3` |
| Part file | `.part` file | `<target>.part` next to the target path; a verified copy writes it and renames it to the target only when it is complete (IMPORT-004) |
| Job | `Job` | Background work in the queue |
| Index | `Index` | Machine-local database, reconstructible from files |
| Rating | `Rating` | 0–5 stars |
| Pick state | `PickState` | Keep / pending / rejected |
| Color label | `ColorLabel` | Color used as a marker |
| Tag | `Tag` | Hierarchical keyword, path with `/` |
| Region | `Region` | Image area, e.g. a face, per MWG |
| Module | `Module` | Business unit as defined in Section 4 |
