# 0024 – A line-based guided import

- Status: accepted
- Date: 2026-11-01

## Context

`framehive import <source> <destination>` (CLI-001) suits scripts, but a person who
comes home with a full memory card has to look up the mount point, remember the
template and type both every time. Before any graphical interface, the most common
task, "import this card", should need no arguments: insert the card, confirm, pull it.

That task needs a dialog: wait for a card (Q1), choose one, ask for a job code (R9),
show what will happen, choose a name template with a preview (N9, text mode), show the
progress, report, eject (A3). The devices service (ADR 0023) already lists and ejects
cards.

A full-screen terminal interface (TUI) would draw this as panes and react to single
keys. It needs a terminal library (`crossterm`, `ratatui` or `termion`), raw mode and
redrawing, which pull in native terminal handling and several crates (ADR 0009), and it
is hard to test without a pseudo-terminal.

## Decision

### 1. `framehive` without arguments starts the guided import

Called without a subcommand, `framehive` runs the guided import (CLI-002, CLI-003). It needs a
person at a terminal: when standard input or standard output is not a terminal
(`std::io::IsTerminal`), it exits with code 2 and points to `framehive import`.

### 2. Lines in, lines out

The dialog prints plain lines to standard output and reads whole lines from standard
input. A choice is a letter or a number followed by Enter; an empty line takes the
default, which every prompt shows. The terminal stays in its normal mode, so editing a
line, copy and paste and Ctrl+C work as everywhere. The only control character written
is a carriage return, which redraws the one progress line in place.

No terminal library and no new dependency is added.

### 3. The dialog is a state machine over input events

The dialog lives in `apps/cli/src/guided/` and is written against `BufRead`-like input
and `Write` output, never against the terminal directly. Its input is a sequence of
events:

- a line the person typed,
- a tick, once a poll interval, while no line arrives, and
- the end of input (Ctrl+D).

On a tick the dialog polls the `DeviceSource` through a `DeviceWatch`, so a card that is
inserted or pulled while the dialog waits for a line is noticed.

The real input reads standard input on a second thread and sends each line over a
channel; the main thread waits for the next line with a timeout of one second and turns
the timeout into a tick. Tests feed a fixed list of events, a `MemoryDeviceSource`, a
memory file system and a fixed clock, and compare the whole output with a golden text.
They never sleep and never touch a terminal (ADR 0006).

### 4. A run can be stopped between files

When a card is pulled during the import, the remaining files cannot be read. Instead
of failing each of them, the import run stops: `ProgressSink` gets a method that the run
asks before each file whether to go on (IMPORT-004). The guided import answers it by
polling the device source, at most once a poll interval, by the clock.

After the card is inserted again, the dialog makes a new plan of the whole card and
executes it. Files that were already copied are then `already imported` (IMPORT-003),
and a file that was cut off left only its `.part` file, which the new run replaces
(IMPORT-004).

## Alternatives

- **A full-screen TUI with `ratatui` and `crossterm`:** nicer to look at and single-key
  choices, but several crates, raw mode, and tests that need a pseudo-terminal or a
  test backend. The graphical interface will be the place for a rich view.
- **Single keys in raw mode with own `termios` code:** needs `libc` and `unsafe` code,
  differs between Linux and macOS, and leaves the terminal broken if the program dies
  in raw mode.
- **Waiting for a card with a blocking read only:** simpler, but a card that is inserted
  while the dialog waits for a line would only be noticed after the person presses
  Enter.
- **Letting the remaining files fail when the card is pulled:** no change to the import
  module, but a pulled card would end in a report with hundreds of failures, and a card
  reader that hangs on a read makes every file wait for its timeout.

## Consequences

- Every choice needs Enter, and the default needs only Enter.
- The guided import runs on any terminal that prints lines, also over SSH.
- The dialog is fully covered by golden tests; a change to any text shows in the diff.
- `framehive` without arguments no longer prints the help; `framehive --help` does.
- `ProgressSink` gets a method with a default answer, so existing sinks keep working.
