use crate::executor::Executor;
use crate::routine::Routine;

#[derive(Default)]
pub struct Parser;

impl Parser {
    pub fn parse(&self, commands: &[String], parallel: bool, verbose: bool) -> Executor {
        let routines = self.parse_commands(commands);
        Executor::new(parallel, verbose, routines)
    }

    fn parse_commands(&self, commands: &[String]) -> Vec<Routine> {
        commands
            .iter()
            .map(|cmd| {
                // Handle commands with arguments
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_space_separated() {
        let parser = Parser::default();
        let commands = vec!["check".to_string(), "test".to_string()];
        let executor = parser.parse(&commands, false, false);

        assert_eq!(executor.routines.len(), 2);

        assert_eq!(executor.routines[0].name, "check");
        assert!(executor.routines[0].args.is_empty());

        assert_eq!(executor.routines[1].name, "test");
        assert!(executor.routines[1].args.is_empty());
    }

    #[test]
    fn test_parse_with_args() {
        let parser = Parser::default();
        let commands = vec![
            "test --features feature1".to_string(),
            "run --release".to_string(),
        ];
        let executor = parser.parse(&commands, false, false);

        assert_eq!(executor.routines.len(), 2);

        assert_eq!(executor.routines[0].name, "test");
        assert_eq!(executor.routines[0].args, vec!["--features", "feature1"]);

        assert_eq!(executor.routines[1].name, "run");
        assert_eq!(executor.routines[1].args, vec!["--release"]);
    }

    #[test]
    fn test_parse_with_spaces() {
        let parser = Parser::default();
        let commands = vec!["check".to_string(), "test".to_string(), "run".to_string()];
        let executor = parser.parse(&commands, false, false);

        assert_eq!(executor.routines.len(), 3);

        assert_eq!(executor.routines[0].name, "check");
        assert!(executor.routines[0].args.is_empty());

        assert_eq!(executor.routines[1].name, "test");
        assert!(executor.routines[1].args.is_empty());

        assert_eq!(executor.routines[2].name, "run");
        assert!(executor.routines[2].args.is_empty());
    }
}
