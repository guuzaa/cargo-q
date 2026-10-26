use super::{run_list, Report, Strategy};
use crate::executor::Options;
use crate::process::Termination;
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
        run_list(routines, progress.as_ref(), options, &report, 0);
        report.outcome()
    }
}
