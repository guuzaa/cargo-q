use crate::executor::Executor;
use crate::routine::Routine;

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Strategy {
    /// Commands are independent (space or ; separator)
    Independent,
    /// Commands are dependent on previous success (& separator)
    Dependent,
    /// Commands pipe output to next command (> separator)
    Pipe,
}

pub struct Parser;

impl Parser {
    pub fn new() -> Self {
        Parser
    }

    pub fn parse(&self, input: &str, parallel: bool, verbose: bool) -> Executor {
        // For now, only implement space separator (Independent strategy)
        let routines = input
            .split_whitespace()
            .map(|cmd| {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                Routine {
                    name: parts[0].to_string(),
                    args: parts[1..].iter().map(|s| s.to_string()).collect(),
                }
            })
            .collect();

        Executor::new(parallel, verbose, routines, Strategy::Independent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_commands() {
        let parser = Parser::new();
        let input = "check test";
        let executor = parser.parse(input, false, false);

        assert_eq!(executor.strategy, Strategy::Independent);
        assert_eq!(executor.routines.len(), 2);

        assert_eq!(executor.routines[0].name, "check");
        assert!(executor.routines[0].args.is_empty());

        assert_eq!(executor.routines[1].name, "test");
        assert!(executor.routines[1].args.is_empty());
    }

    #[test]
    fn test_parse_simple_one_command() {
        let parser = Parser::new();
        let input = "check";
        let executor = parser.parse(input, false, false);

        assert_eq!(executor.strategy, Strategy::Independent);
        assert_eq!(executor.routines.len(), 1);

        assert_eq!(executor.routines[0].name, "check");
        assert!(executor.routines[0].args.is_empty());
    }
}
