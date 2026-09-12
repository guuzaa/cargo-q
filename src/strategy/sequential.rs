use super::{run_one, Report, Strategy};
use crate::executor::Options;
use crate::process::{self, Termination};
use crate::progress::Progress;
use crate::routine::Routine;
use std::io;
use std::sync::Arc;

pub struct Sequential;

impl Strategy for Sequential {
    /// Run the routines one after another, stopping at the first failure
    /// unless `--keep-going` was asked for.
    fn execute(
        &self,
        routines: &[Routine],
        progress: &Arc<dyn Progress>,
        options: Options,
    ) -> io::Result<Termination> {
        let report = Report::default();

        for (id, routine) in routines.iter().enumerate() {
            if process::was_interrupted() {
                break;
            }
            if !run_one(id, routine, progress.as_ref(), options, &report) {
                break;
            }
        }

        report.outcome()
    }
}
