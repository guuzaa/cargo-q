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
        // Check for & first as it's the most restrictive
        if input.contains('&') {
            let routines = self.parse_ampersand_separated(input);
            Executor::new(parallel, verbose, routines, Strategy::Dependent)
        } else if input.contains(';') {
            let routines = self.parse_semicolon_separated(input);
            Executor::new(parallel, verbose, routines, Strategy::Independent)
        } else {
            let routines = self.parse_space_separated(input);
            Executor::new(parallel, verbose, routines, Strategy::Independent)
        }
    }

    fn parse_space_separated(&self, input: &str) -> Vec<Routine> {
        input
            .split_whitespace()
            .map(|cmd| self.create_routine(cmd))
            .collect()
    }

    fn parse_semicolon_separated(&self, input: &str) -> Vec<Routine> {
        input
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|cmd| self.create_routine(cmd))
            .filter(|routine| !routine.is_empty())
            .collect()
    }

    fn parse_ampersand_separated(&self, input: &str) -> Vec<Routine> {
        input
            .split('&')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|cmd| self.create_routine(cmd))
            .filter(|routine| !routine.is_empty())
            .collect()
    }

    fn create_routine(&self, cmd: &str) -> Routine {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Routine::default();
        }
        Routine {
            name: parts[0].to_string(),
            args: parts[1..].iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_space_separated() {
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
    fn test_parse_semicolon_separated() {
        let parser = Parser::new();
        let input = "test --features feature1 ; run";
        let executor = parser.parse(input, false, false);

        assert_eq!(executor.strategy, Strategy::Independent);
        assert_eq!(executor.routines.len(), 2);

        assert_eq!(executor.routines[0].name, "test");
        assert_eq!(executor.routines[0].args, vec!["--features", "feature1"]);

        assert_eq!(executor.routines[1].name, "run");
        assert!(executor.routines[1].args.is_empty());
    }

    #[test]
    fn test_parse_ampersand_separated() {
        let parser = Parser::new();
        let input = "check & test & run";
        let executor = parser.parse(input, false, false);

        assert_eq!(executor.strategy, Strategy::Dependent);
        assert_eq!(executor.routines.len(), 3);

        assert_eq!(executor.routines[0].name, "check");
        assert!(executor.routines[0].args.is_empty());

        assert_eq!(executor.routines[1].name, "test");
        assert!(executor.routines[1].args.is_empty());

        assert_eq!(executor.routines[2].name, "run");
        assert!(executor.routines[2].args.is_empty());
    }

    #[test]
    fn test_parse_ampersand_with_args() {
        let parser = Parser::new();
        let input = "test --features feature1 & run --release";
        let executor = parser.parse(input, false, false);

        assert_eq!(executor.strategy, Strategy::Dependent);
        assert_eq!(executor.routines.len(), 2);

        assert_eq!(executor.routines[0].name, "test");
        assert_eq!(executor.routines[0].args, vec!["--features", "feature1"]);

        assert_eq!(executor.routines[1].name, "run");
        assert_eq!(executor.routines[1].args, vec!["--release"]);
    }

    #[test]
    fn test_parse_ampersand_without_spaces() {
        let parser = Parser::new();
        let input = "check&test&run";
        let executor = parser.parse(input, false, false);

        assert_eq!(executor.strategy, Strategy::Dependent);
        assert_eq!(executor.routines.len(), 3);

        assert_eq!(executor.routines[0].name, "check");
        assert!(executor.routines[0].args.is_empty());

        assert_eq!(executor.routines[1].name, "test");
        assert!(executor.routines[1].args.is_empty());

        assert_eq!(executor.routines[2].name, "run");
        assert!(executor.routines[2].args.is_empty());
    }

    #[test]
    fn test_parse_semicolon_with_empty() {
        let parser = Parser::new();
        let input = "check ; ; test";
        let executor = parser.parse(input, false, false);

        assert_eq!(executor.strategy, Strategy::Independent);
        assert_eq!(executor.routines.len(), 2);

        assert_eq!(executor.routines[0].name, "check");
        assert!(executor.routines[0].args.is_empty());

        assert_eq!(executor.routines[1].name, "test");
        assert!(executor.routines[1].args.is_empty());
    }

    #[test]
    fn test_parse_semicolon_with_spaces() {
        let parser = Parser::new();
        let input = "  check  ;  test  ;  run  ";
        let executor = parser.parse(input, false, false);

        assert_eq!(executor.strategy, Strategy::Independent);
        assert_eq!(executor.routines.len(), 3);

        assert_eq!(executor.routines[0].name, "check");
        assert!(executor.routines[0].args.is_empty());

        assert_eq!(executor.routines[1].name, "test");
        assert!(executor.routines[1].args.is_empty());

        assert_eq!(executor.routines[2].name, "run");
        assert!(executor.routines[2].args.is_empty());
    }
}
