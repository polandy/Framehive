# Getting started

## Import a memory card, guided

Run `framehive` without a command in a terminal and insert the card:

```sh
framehive
```

It waits for the card, reads it, and asks step by step: the folder to import into, the
name template (with a preview of the first names), the job code, and the backup folders
(`b` at the summary). Enter starts the import. It shows the progress on one line, and at
the end the report and whether the card can be formatted. Then it offers to eject the
card, or for a camera to release it. Your answers are remembered for the next card.

## Import from the command line

`framehive import` does the same without questions, for scripts and for folders:

```sh
framehive import /media/card ~/Pictures/Library
```

The last folder is the destination, every folder before it a source. Each file is
copied into a folder by its capture date, verified, and skipped when it was imported
before. Try a plan first; it copies nothing:

```sh
framehive import /media/card ~/Pictures/Library --dry-run
```

A few common variations:

```sh
# Keep a second copy on a backup disk
framehive import /media/card ~/Pictures/Library --backup /mnt/backup/Photos

# Choose the folders and names, and add a job code
framehive import /media/card ~/Pictures/Library \
  --template '{date:%Y}/{date:%Y-%m-%d}_{job}/{date:%Y%m%d}-{seq_today:4}.{ext}' \
  --job-code wedding-smith

# Put videos in a folder of their own
framehive import /media/card ~/Pictures/Library --video-destination ~/Videos/Library

# Import from the only connected camera
framehive import camera: ~/Pictures/Library
```

The tokens of the name template are in [Commands](commands.md#name-templates).

## Save the settings you always use

Settings you give every time belong in an import profile. `framehive config init`
writes an example to start from; then use it by name:

```sh
framehive config init
framehive import /media/card ~/Pictures/Library --profile example
```

See [Configuration](configuration.md) for the keys.

## Look back at an import

Every verified copy is recorded in the import history:

```sh
framehive import-log          # one line per run
framehive import-log last     # which file of the last run went where
```

## Before you format the card

Read the last lines of the report. Format the card only when it says
`The card can be formatted`. [Safety questions](safety.md) explains what that answer
checks and what it does not.
