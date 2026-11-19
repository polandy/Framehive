# Safety questions

## When can I format the memory card?

When the last lines of the report say `The card can be formatted` (or `The source can
be formatted` for `framehive import`). That answer means every photo and video of the
card is verified at every place this run was asked to save it: the destination and each
backup folder. Verified means copied and then read back with a matching checksum, or
found there already with the same content.

When it says `Do not format the card`, the lines below it name each file that is not
saved yet, and in how many of its places it is verified. Fix the cause and run the same
import again: files already saved are not copied twice, and only the rest is done.

A file that is not a photo or video, hidden, or a symbolic link is left out and does not
keep the card from being formatted; it is not saved either. A file that cannot be read
does keep it. From a camera, only the files below `DCIM` are checked; everything else on
a phone is not backed up.

## What does the answer not check?

- **Your library after the import.** A file *imported before* counts as saved at the
  destination because the import history says so; the destination is not looked at
  again. If you deleted or moved it in the library since, the card may hold the last
  copy.
- **Backup folders are looked at.** A file *backed up before* counts only while a file of
  the same size is still at its place in the backup folder. A swapped or emptied backup
  disk is therefore filled again by the next import.
- **The disk itself.** The check reads the copy back after it has been written, but the
  operating system may answer from its memory cache rather than from the disk. A failing
  USB stick or network drive can still lose data later. Keep more than one backup.
- **Places you did not name.** With no `--backup`, the destination is the only place, so
  "can be formatted" then means one copy.

## Does Framehive ever delete my photos?

The import never deletes or changes a file on the source, and it never replaces a file at
the destination: a different file under the same name gets a new name
(`DSCF0001-1.RAF`) or is skipped, as `--on-conflict` says. A copy is written under a
temporary `.part` name and renamed only after it is complete.

`framehive library remove-raws` is the only command that removes files, and it moves them
to the trash of the system, where you can restore them ([Where Framehive keeps its
files](locations.md#trash)).

## What does Remove RAWs keep?

For a RAW + JPEG pair in the folder, it moves the RAW and the RAW's own sidecar
(`DSCF1234.RAF.xmp`) to the trash. It keeps:

- the JPEG, its sidecar and a shared `DSCF1234.xmp`, with all ratings and keywords;
- every asset marked Keep RAW (`framehive library keep-raw <file>`);
- every rejected asset;
- a RAW without a JPEG, and a JPEG without a RAW;
- the files in subfolders.

Run it with `--dry-run` first to see the list. It always asks before moving anything,
unless `--yes` is given.

## Can I run two imports at once?

Yes. `framehive import` with several sources runs them in parallel where they share no
disk, and two `framehive` commands may run side by side: the running numbers are taken
under a lock, and each copy is checked against what is already at its target.
