# Configuration

Framehive reads two kinds of YAML files from its [configuration folder](locations.md):
`config.yaml` with the settings for every import, and import profiles in `profiles/`
with the settings of one kind of import. Both are optional. Each file starts with
`schema_version`. A key Framehive does not know, a key given twice, or a value of the
wrong kind is an error that names the file, the line and the column, often with a
suggestion such as `did you mean destinations?`. Nothing is imported until the file is
fixed. The JSON Schemas in [`schemas/`](../../schemas/) describe the same keys, for
editors that check YAML against a schema.

## `config.yaml`

The metadata that every import writes into the XMP sidecar of each photo, in the guided
import and in `framehive import`:

```yaml
schema_version: 1
metadata:
  creator:
    - Ada Lovelace
  copyright: "© {year} Ada Lovelace"
  keywords:
    - Holiday/Beach
  sidecar_naming: original_with_extension
```

| Key | Meaning | Default |
|---|---|---|
| `metadata.creator` | Creators of the photos, in this order | none |
| `metadata.copyright` | Copyright notice; `{year}` becomes the year the photo was taken | none |
| `metadata.keywords` | Keywords; `/` separates the levels, as in `Holiday/Beach` | none |
| `metadata.sidecar_naming` | `original_with_extension` (`DSCF0042.RAF.xmp`) or `original_without_extension` (`DSCF0042.xmp`) | `original_with_extension` |

A sidecar that a camera or another program already wrote keeps its creator and
copyright, gets the configured keywords added to its own, and gets the import's job code
in place of its own. The report names each file where a value was kept or replaced.

## Import profiles

A profile holds the settings you would otherwise give on every `framehive import`, under
a name: `profiles/holiday.yaml` is used with `--profile holiday`. A name has 1 to 64 of
`a-z`, `0-9`, `-` and `_`. `framehive config init` writes `profiles/example.yaml` with
every key and a comment for each.

```yaml
schema_version: 3
naming:
  template: "{date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}"
  on_conflict: add_suffix
job_codes:
  - holiday-2026
backup:
  destinations:
    - /mnt/backup/Photos
video_destination: ~/Videos/Library
metadata:
  keywords:
    - Holiday
```

| Key | Meaning | Command-line option |
|---|---|---|
| `naming.template` | Name template ([tokens](commands.md#name-templates)) | `--template` |
| `naming.video_template` | Name template for videos | `--video-template` |
| `naming.on_conflict` | `add_suffix`, `skip` or `fail` | `--on-conflict` |
| `job_codes` | Job codes; the first one is used | `--job-code` |
| `backup.destinations` | Backup folders, in this order | `--backup` |
| `backup.video_destinations` | Backup folders for videos | `--video-backup` |
| `video_destination` | Folder for videos | `--video-destination` |
| `metadata.*` | As in `config.yaml`; a key set here replaces that key of `config.yaml` | none |
| `source_stall_limit_seconds` | Seconds, 1 to 3600 (default 30), a source may take to answer one read or listing before that file and the rest of the source fail; needs `schema_version: 3` | none |

An option on the command line replaces the profile's value; `--backup` and
`--video-backup` replace the whole list. A profile never names the destination: that is
always given on the command line. `~` in a folder is your home folder. A profile with
`schema_version: 1` is still read.
