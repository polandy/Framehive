// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The diagnostic output on standard error, as detailed as `--verbose` asks
//! (CLI-021, ADR 0046).

use std::io;

use tracing::Subscriber;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::MakeWriter;

/// Writes the diagnostic events of this process to standard error from now on.
pub(crate) fn start(verbose: u8) {
    // It fails only when a subscriber is set already, and nothing else sets one.
    let _ = tracing::subscriber::set_global_default(subscriber(verbose, io::stderr));
}

/// The most detailed level shown when `--verbose` was given `verbose` times.
fn level(verbose: u8) -> LevelFilter {
    match verbose {
        0 => LevelFilter::WARN,
        1 => LevelFilter::INFO,
        2 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    }
}

/// A subscriber that writes each event up to the level of `verbose` as one line to
/// `writer`, without a time, so that the lines of two runs compare.
fn subscriber<W>(verbose: u8, writer: W) -> impl Subscriber + Send + Sync
where
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_max_level(level(verbose))
        .without_time()
        .finish()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests may panic")]

    use std::sync::{Arc, Mutex};

    use super::*;

    /// Collects what the subscriber writes.
    #[derive(Clone, Default)]
    struct Collected(Arc<Mutex<Vec<u8>>>);

    impl io::Write for Collected {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Collected {
        fn text(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    /// The lines written while `emit` runs with `--verbose` given `verbose` times.
    fn written(verbose: u8, emit: impl FnOnce()) -> String {
        let collected = Collected::default();
        let writer = collected.clone();
        tracing::subscriber::with_default(subscriber(verbose, move || writer.clone()), emit);
        collected.text()
    }

    fn one_of_each_level() {
        tracing::warn!("a warning");
        tracing::info!("some information");
        tracing::debug!("a detail");
        tracing::trace!("a trace");
    }

    /// spec: CLI-021.1
    #[test]
    fn shows_only_warnings_without_verbose() {
        let text = written(0, one_of_each_level);

        assert!(text.contains("WARN"), "{text}");
        assert!(text.contains("a warning"), "{text}");
        assert!(!text.contains("some information"), "{text}");
    }

    /// spec: CLI-021.2
    #[test]
    fn verbose_adds_information_and_twice_details() {
        let once = written(1, one_of_each_level);
        let twice = written(2, one_of_each_level);

        assert!(once.contains("some information"), "{once}");
        assert!(!once.contains("a detail"), "{once}");
        assert!(twice.contains("a detail"), "{twice}");
        assert!(!twice.contains("a trace"), "{twice}");
    }

    /// spec: CLI-021.2
    #[test]
    fn three_times_verbose_shows_everything() {
        let text = written(3, one_of_each_level);

        assert!(text.contains("a trace"), "{text}");
    }

    /// spec: CLI-021.3
    #[test]
    fn a_line_has_no_time_and_names_its_spans() {
        let text = written(1, || {
            let _job = tracing::info_span!("import_job", source = "/card").entered();
            tracing::info!("started");
        });

        assert_eq!(
            text,
            " INFO import_job{source=\"/card\"}: framehive::diagnostics::tests: started\n"
        );
    }
}
