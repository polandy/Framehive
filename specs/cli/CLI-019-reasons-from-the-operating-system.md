# CLI-019 – Reasons from the operating system

- Status: implemented
- E2e-owed: 2, 3
- Module: cli (`apps/cli`)
- Origin: N18
- Depends on: CLI-001

## Purpose

An error message that ends in `No such file or directory (os error 2)` reads like a
crash. Every `framehive` command names the reason in plain words instead (review E26).

## Behaviour

When an error message names a reason that comes from the operating system, the reason
has no error number. Common reasons have fixed words:

| Reason | Words |
|---|---|
| The path does not exist | `it does not exist` |
| Access is denied | `permission denied` |
| No space is left | `the disk is full` |
| The file system is mounted read-only | `the file system is read-only` |
| A part of the path is a file | `a part of the path is not a folder` |

Any other reason is the system's text with a lower-case first letter and without
` (os error <n>)`. A reason that Framehive itself names keeps its words.

## Acceptance criteria

### CLI-019.1 – The command names the reason in plain words
- **Given** the source of CLI-001.1 and a file `<file>`
- **When** `framehive import <source> <file>/library` runs
- **Then** standard error has `error: the destination <file>/library cannot be read:
  a part of the path is not a folder`, without an error number, and the exit code is 2

### CLI-019.2 – Common reasons have fixed words
- **Given** each reason of the table as an error of the operating system
- **When** it is named
- **Then** it has the words of the table

### CLI-019.3 – Any other reason loses its error number
- **Given** the error `Is a directory (os error 21)`
- **When** it is named
- **Then** it reads `is a directory`

## Out of scope

- The hints that follow some errors: the spec of each command.

## History

| Date | Change |
|---|---|
| 2026-11-19 | Created (review E26) |
