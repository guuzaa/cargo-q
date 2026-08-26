use crate::routine::Routine;
use crate::strategy::{ExecutionStrategy, ParallelStrategy, SequentialStrategy};
use std::io;

pub(crate) struct Executor {
    pub(super) parallel: bool,
    pub(super) verbose: bool,
    pub(super) routines: Vec<Routine>,
}

impl Executor {
    pub fn new(commands: &[String], parallel: bool, verbose: bool) -> Self {
        Executor {
            parallel,
            verbose,
            routines: Self::parse_commands(commands),
        }
    }

    fn parse_commands(commands: &[String]) -> Vec<Routine> {
        commands
            .iter()
            .map(|cmd| {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.is_empty() {
                    return Routine::default();
                }

                Routine {
                    name: parts[0].to_string(),
                    args: parts[1..].iter().map(|s| s.to_string()).collect(),
                }
            })
            .collect()
    }

    pub fn execute(&self) -> io::Result<()> {
        let strategy: Box<dyn ExecutionStrategy> = match self.parallel {
            true => Box::new(ParallelStrategy),
            false => Box::new(SequentialStrategy),
        };

        strategy.execute(&self.routines, self.verbose)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_space_separated() {
        let commands = vec!["check".to_string(), "test".to_string()];
        let executor = Executor::new(&commands, false, false);

        assert_eq!(executor.routines.len(), 2);

        assert_eq!(executor.routines[0].name, "check");
        assert!(executor.routines[0].args.is_empty());

        assert_eq!(executor.routines[1].name, "test");
        assert!(executor.routines[1].args.is_empty());
    }

    #[test]
    fn test_parse_with_args() {
        let commands = vec![
            "test --features feature1".to_string(),
            "run --release".to_string(),
        ];
        let executor = Executor::new(&commands, false, false);

        assert_eq!(executor.routines.len(), 2);

        assert_eq!(executor.routines[0].name, "test");
        assert_eq!(executor.routines[0].args, vec!["--features", "feature1"]);

        assert_eq!(executor.routines[1].name, "run");
        assert_eq!(executor.routines[1].args, vec!["--release"]);
    }

    #[test]
    fn test_parse_with_spaces() {
        let commands = vec!["check".to_string(), "test".to_string(), "run".to_string()];
        let executor = Executor::new(&commands, false, false);

        assert_eq!(executor.routines.len(), 3);

        assert_eq!(executor.routines[0].name, "check");
        assert!(executor.routines[0].args.is_empty());

        assert_eq!(executor.routines[1].name, "test");
        assert!(executor.routines[1].args.is_empty());

        assert_eq!(executor.routines[2].name, "run");
        assert!(executor.routines[2].args.is_empty());
    }
}
