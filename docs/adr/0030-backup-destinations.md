# 0030 – Backup destinations

- Status: accepted
- Date: 2026-11-04
- Amends: ADR 0020, decision 3, in how a verified copy writes several targets; ADR 0025,
  decision 2, in the keys of the state file

## Context

The import writes each file to backup destinations as well (I2, IMPORT-007), with the
same target path as in the import destination. ADR 0020 defines a verified copy for one
target: the source is read once, its checksum computed while the bytes go to
`<target>.part`, and the target is read back and compared. ADR 0025 defines the state
file of the guided import with the keys `destination`, `template` and `job_code`.

Reading a memory card is often the slowest part of an import. A card read once per
destination would double or triple the time with one or two backup drives, and each read
is another chance for a flaky card reader to return other bytes.

## Decision

### 1. One read, one checksum, several targets

The `files` service gets a verified copy to several targets. It reads the source once,
computes the checksum while it reads, and writes the same bytes to the `.part` file of
each target. Each target then goes through the steps of ADR 0020 on its own: sync,
rename without replacing, sync the folder, read back and compare with the source
checksum. The result is one outcome per target. A failure at one target removes only
that target's `.part` file or unverified target, and the bytes keep going to the other
targets.

A target whose `.part` file fails while it is written is dropped from the rest of the
read; the read ends early only when every target failed. The one-target
`copy_verified` stays and is the same copy with one target.

### 2. Backups never overwrite

Before the copy, the import checks each backup target. A file with identical content is
"already backed up" and is not written; a file with other content is a failure of that
backup destination. There is no overwrite policy for backups, as there is none for the
import destination (ADR 0020, decision 2).

### 3. The state file remembers backup destinations

The state file of the guided import gets the key `backup`, which repeats, in the order
the backup destinations are used. Its value follows the rules of `destination`: a path
that is not valid UTF-8 or contains a line break is not remembered. The format version
becomes 2:

```
# Framehive remembers the answers of the guided import here.
format_version=2
destination=/home/ada/Pictures
backup=/run/media/ada/Backup/Pictures
template={date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}
job_code=Smith wedding
```

A file with `format_version=1` is still read; it has no backup destinations. The next
write stores version 2. A Framehive that knows only version 1 ignores a version-2 file
with a warning, as ADR 0025, decision 3, describes.

## Alternatives

- **Copy to the import destination, then from there to each backup:** reads the card
  only once too, but each backup then depends on a copy that is itself only verified
  against the source, and a slow library disk is read again for every backup.
- **Read the source once per destination:** the simplest code, but doubles the time
  spent on the card and the chance of a bad read.
- **An overwrite policy for backups, as RPD offers (I5):** a backup folder that holds
  other content under the same name is a sign of a mix-up, and overwriting it could
  destroy the only copy of an earlier photo.
- **Keep `format_version=1` and add the key:** an older Framehive would then reject a
  file of the same version, which hides what changed.

## Consequences

- A slow backup drive slows the whole import, because all targets are written in step.
- A file that fails at the import destination can still be verified at a backup
  destination; a re-run then copies it to the import destination and finds it already
  backed up.
- The "card can be formatted" status (N5) can count verified copies per file from the
  outcomes of one run.
