//! Running the same list of routines more than once.

use super::{run_list, Report, Strategy};
use crate::executor::Options;
use crate::process::Termination;
use crate::progress::Progress;
use crate::routine::Routine;
use std::io;
use std::sync::Arc;

/// Run the whole list of routines, once per round.
///
/// A round is the whole list, in the order the commands were given, so
/// `--repeat 3 check test` runs `check test` three times over.
///
/// The rounds run one after another: `--repeat` and `--parallel` are
/// mutually exclusive.
pub struct Repeat;

impl Strategy for Repeat {
    /// Run the list once per round, stopping at the first failure unless
    /// `--keep-going` was asked for.
    ///
    /// Every round shares one report, so the run still ends with the first
    /// failure's exit code, whichever round it happened in.
    fn execute(
        &self,
        routines: &[Routine],
        progress: &Arc<dyn Progress>,
        options: Options,
    ) -> io::Result<Termination> {
        let report = Report::default();
        run(routines, progress.as_ref(), options, &report);
        report.outcome()
    }
}

/// Run the list once per round.
///
/// The number of rounds comes from `options.repeat`; a list of zero rounds
/// would run nothing at all, so there is always at least one.
///
/// Every round shares one report, so the run still ends with the first
/// failure's exit code, whichever round it happened in.
///
/// Returns whether the caller may run more rounds: a failure without
/// `--keep-going`, an interrupt, or a command that could not be started
/// ends the run.
fn run(routines: &[Routine], progress: &dyn Progress, options: Options, report: &Report) -> bool {
    let rounds = options.repeat.max(1);

    for round in 0..rounds {
        // Each round numbers its commands from its own offset, so every
        // run keeps a slot of its own in the progress display.
        let offset = round * routines.len();
        if !run_list(routines, progress, options, report, offset) {
            return false;
        }
    }

    true
}
