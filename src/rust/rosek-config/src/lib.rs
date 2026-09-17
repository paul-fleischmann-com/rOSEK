//! Parser for rOSEK's OIL-inspired static configuration format.
//!
//! Host-only (`std`), never compiled for the `armv7a-none-eabi` target —
//! used from `rosek-board-beaglebone-black/build.rs` (issue #9) and
//! unit-tested on the host toolchain (issue #10, PLAN.md
//! "Testinfrastruktur": "Config-Parser/Codegenerator: normale
//! Host-Unit-Tests, laufen ohne Zielhardware").
//!
//! Grammar decided in issue #7, documented in full in
//! docs/arc42/08-concepts.adoc:
//!
//! ```text
//! file       := { block } ;
//! block      := keyword name "{" { attribute } "}" ";" ;
//! keyword    := "TASK" | "OS_APPLICATION" ;
//! attribute  := key "=" value ";" ;
//! value      := integer | ident | "TRUE" | "FALSE" ;
//! comment    := "//" { any-character-until-newline } ;
//! ```
//!
//! This crate covers syntactic parsing and per-attribute value validation
//! (e.g. `SCHEDULE` must be `FULL`/`NON`) only. Cross-references between
//! blocks (does a Task's `APPLICATION` actually name a declared
//! `OS_APPLICATION`?) are a semantic, codegen-time concern — issue #9's
//! job, not this one's.

pub mod codegen;
mod lexer;

use lexer::{Lexer, Token};
use std::fmt;

/// A fully parsed `.cfg` file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub tasks: Vec<Task>,
    pub os_applications: Vec<OsApplication>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub name: String,
    pub priority: u32,
    pub schedule: Schedule,
    pub application: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schedule {
    Full,
    Non,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsApplication {
    pub name: String,
    pub trusted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parses a complete `.cfg` source string into a [`Config`].
pub fn parse(source: &str) -> Result<Config, ParseError> {
    Parser::new(source).parse_file()
}

struct Parser<'a> {
    tokens: Vec<(Token<'a>, usize)>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Parser {
            tokens: Lexer::new(source).collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<&Token<'a>> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    fn line(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map(|(_, l)| *l)
            .or_else(|| self.tokens.last().map(|(_, l)| *l))
            .unwrap_or(1)
    }

    fn advance(&mut self) -> Option<Token<'a>> {
        let tok = self.tokens.get(self.pos).map(|(t, _)| t.clone());
        self.pos += 1;
        tok
    }

    fn err(&self, message: impl Into<String>) -> ParseError {
        self.err_at(self.line(), message)
    }

    fn err_at(&self, line: usize, message: impl Into<String>) -> ParseError {
        ParseError {
            line,
            message: message.into(),
        }
    }

    fn expect_ident(&mut self, what: &str) -> Result<&'a str, ParseError> {
        match self.advance() {
            Some(Token::Ident(s)) => Ok(s),
            other => Err(self.err(format!("expected {what}, found {other:?}"))),
        }
    }

    fn expect_lbrace(&mut self) -> Result<(), ParseError> {
        match self.advance() {
            Some(Token::LBrace) => Ok(()),
            other => Err(self.err(format!("expected '{{', found {other:?}"))),
        }
    }

    fn expect_equals(&mut self) -> Result<(), ParseError> {
        match self.advance() {
            Some(Token::Equals) => Ok(()),
            other => Err(self.err(format!("expected '=', found {other:?}"))),
        }
    }

    fn expect_semicolon(&mut self, what: &str) -> Result<(), ParseError> {
        match self.advance() {
            Some(Token::Semicolon) => Ok(()),
            other => Err(self.err(format!("expected ';' {what}, found {other:?}"))),
        }
    }

    fn parse_file(mut self) -> Result<Config, ParseError> {
        let mut config = Config::default();
        while self.peek().is_some() {
            match self.peek() {
                Some(Token::Ident("TASK")) => {
                    self.advance();
                    config.tasks.push(self.parse_task()?);
                }
                Some(Token::Ident("OS_APPLICATION")) => {
                    self.advance();
                    config.os_applications.push(self.parse_os_application()?);
                }
                other => {
                    return Err(
                        self.err(format!("expected TASK or OS_APPLICATION, found {other:?}"))
                    )
                }
            }
        }
        Ok(config)
    }

    /// Returns each attribute together with the line its *key* appeared
    /// on, so callers can report semantic errors (unknown/invalid
    /// attribute) at the attribute's own location instead of wherever
    /// parsing happens to be by the time the whole block is done.
    fn parse_attributes(&mut self) -> Result<Vec<(&'a str, AttrValue<'a>, usize)>, ParseError> {
        self.expect_lbrace()?;
        let mut attrs = Vec::new();
        loop {
            match self.peek() {
                Some(Token::RBrace) => {
                    self.advance();
                    break;
                }
                Some(Token::Ident(_)) => {
                    let attr_line = self.line();
                    let key = self.expect_ident("attribute name")?;
                    self.expect_equals()?;
                    let value = match self.advance() {
                        Some(Token::Number(n)) => AttrValue::Number(n),
                        Some(Token::Ident(id)) => AttrValue::Ident(id),
                        other => return Err(self.err(format!("expected a value, found {other:?}"))),
                    };
                    self.expect_semicolon("after attribute")?;
                    attrs.push((key, value, attr_line));
                }
                other => {
                    return Err(self.err(format!("expected an attribute or '}}', found {other:?}")))
                }
            }
        }
        self.expect_semicolon("after block")?;
        Ok(attrs)
    }

    fn parse_task(&mut self) -> Result<Task, ParseError> {
        let name = self.expect_ident("task name")?.to_string();
        let attrs = self.parse_attributes()?;

        let mut priority = None;
        let mut schedule = None;
        let mut application = None;
        for (key, value, line) in attrs {
            match key {
                "PRIORITY" => priority = Some(expect_number(&value, key, self, line)?),
                "SCHEDULE" => schedule = Some(expect_schedule(&value, self, line)?),
                "APPLICATION" => {
                    application = Some(expect_ident_value(&value, key, self, line)?.to_string())
                }
                other => return Err(self.err_at(line, format!("unknown TASK attribute '{other}'"))),
            }
        }

        Ok(Task {
            priority: priority
                .ok_or_else(|| self.err(format!("TASK {name} is missing PRIORITY")))?,
            schedule: schedule
                .ok_or_else(|| self.err(format!("TASK {name} is missing SCHEDULE")))?,
            application: application
                .ok_or_else(|| self.err(format!("TASK {name} is missing APPLICATION")))?,
            name,
        })
    }

    fn parse_os_application(&mut self) -> Result<OsApplication, ParseError> {
        let name = self.expect_ident("OS_APPLICATION name")?.to_string();
        let attrs = self.parse_attributes()?;

        let mut trusted = None;
        for (key, value, line) in attrs {
            match key {
                "TRUSTED" => trusted = Some(expect_bool(&value, self, line)?),
                other => {
                    return Err(
                        self.err_at(line, format!("unknown OS_APPLICATION attribute '{other}'"))
                    )
                }
            }
        }

        Ok(OsApplication {
            trusted: trusted
                .ok_or_else(|| self.err(format!("OS_APPLICATION {name} is missing TRUSTED")))?,
            name,
        })
    }
}

enum AttrValue<'a> {
    Number(u32),
    Ident(&'a str),
}

fn expect_number(
    value: &AttrValue<'_>,
    key: &str,
    p: &Parser<'_>,
    line: usize,
) -> Result<u32, ParseError> {
    match value {
        AttrValue::Number(n) => Ok(*n),
        AttrValue::Ident(_) => Err(p.err_at(line, format!("{key} must be a number"))),
    }
}

fn expect_ident_value<'a>(
    value: &AttrValue<'a>,
    key: &str,
    p: &Parser<'_>,
    line: usize,
) -> Result<&'a str, ParseError> {
    match value {
        AttrValue::Ident(id) => Ok(id),
        AttrValue::Number(_) => Err(p.err_at(line, format!("{key} must be an identifier"))),
    }
}

fn expect_schedule(
    value: &AttrValue<'_>,
    p: &Parser<'_>,
    line: usize,
) -> Result<Schedule, ParseError> {
    match value {
        AttrValue::Ident("FULL") => Ok(Schedule::Full),
        AttrValue::Ident("NON") => Ok(Schedule::Non),
        _ => Err(p.err_at(line, "SCHEDULE must be FULL or NON")),
    }
}

fn expect_bool(value: &AttrValue<'_>, p: &Parser<'_>, line: usize) -> Result<bool, ParseError> {
    match value {
        AttrValue::Ident("TRUE") => Ok(true),
        AttrValue::Ident("FALSE") => Ok(false),
        _ => Err(p.err_at(line, "expected TRUE or FALSE")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_example_from_the_grammar_decision() {
        let src = r#"
            TASK MainTask {
              PRIORITY = 5;
              SCHEDULE = FULL;
              APPLICATION = TrustedApp;
            };

            OS_APPLICATION TrustedApp {
              TRUSTED = TRUE;
            };
        "#;
        let config = parse(src).expect("should parse");
        assert_eq!(
            config.tasks,
            vec![Task {
                name: "MainTask".into(),
                priority: 5,
                schedule: Schedule::Full,
                application: "TrustedApp".into(),
            }]
        );
        assert_eq!(
            config.os_applications,
            vec![OsApplication {
                name: "TrustedApp".into(),
                trusted: true,
            }]
        );
    }

    #[test]
    fn parses_non_preemptive_and_non_trusted() {
        let src = r#"
            TASK Bg { PRIORITY = 0; SCHEDULE = NON; APPLICATION = Untrusted; };
            OS_APPLICATION Untrusted { TRUSTED = FALSE; };
        "#;
        let config = parse(src).unwrap();
        assert_eq!(config.tasks[0].schedule, Schedule::Non);
        assert!(!config.os_applications[0].trusted);
    }

    #[test]
    fn skips_line_comments() {
        let src = r#"
            // this is a comment
            TASK T { // trailing comment
              PRIORITY = 1; // another one
              SCHEDULE = FULL;
              APPLICATION = A;
            };
        "#;
        let config = parse(src).unwrap();
        assert_eq!(config.tasks.len(), 1);
    }

    #[test]
    fn empty_file_parses_to_empty_config() {
        assert_eq!(parse("").unwrap(), Config::default());
        assert_eq!(
            parse("   \n // just a comment\n").unwrap(),
            Config::default()
        );
    }

    #[test]
    fn multiple_blocks() {
        let src = r#"
            TASK A { PRIORITY = 1; SCHEDULE = FULL; APPLICATION = App1; };
            TASK B { PRIORITY = 2; SCHEDULE = NON; APPLICATION = App2; };
            OS_APPLICATION App1 { TRUSTED = TRUE; };
            OS_APPLICATION App2 { TRUSTED = FALSE; };
        "#;
        let config = parse(src).unwrap();
        assert_eq!(config.tasks.len(), 2);
        assert_eq!(config.os_applications.len(), 2);
    }

    #[test]
    fn rejects_unknown_top_level_keyword() {
        let err = parse("RESOURCE Foo { };").unwrap_err();
        assert!(err.message.contains("TASK or OS_APPLICATION"));
    }

    #[test]
    fn rejects_unknown_task_attribute() {
        let err = parse("TASK T { FOO = 1; };").unwrap_err();
        assert!(err.message.contains("unknown TASK attribute"));
    }

    #[test]
    fn rejects_invalid_schedule_value() {
        let err =
            parse("TASK T { PRIORITY = 1; SCHEDULE = MAYBE; APPLICATION = A; };").unwrap_err();
        assert!(err.message.contains("FULL or NON"));
    }

    #[test]
    fn rejects_missing_required_attribute() {
        let err = parse("TASK T { PRIORITY = 1; SCHEDULE = FULL; };").unwrap_err();
        assert!(err.message.contains("missing APPLICATION"));
    }

    #[test]
    fn rejects_missing_semicolon_after_attribute() {
        let err = parse("TASK T { PRIORITY = 1 };").unwrap_err();
        assert!(err.message.contains("';'"));
    }

    #[test]
    fn rejects_missing_semicolon_after_block() {
        let err = parse("TASK T { PRIORITY = 1; SCHEDULE = FULL; APPLICATION = A; }").unwrap_err();
        assert!(err.message.contains("';' after block"));
    }

    #[test]
    fn error_reports_correct_line_number() {
        let src = "TASK T {\n  PRIORITY = 1;\n  FOO = 2;\n};";
        let err = parse(src).unwrap_err();
        assert_eq!(err.line, 3);
    }

    #[test]
    fn priority_must_be_a_number() {
        let err =
            parse("TASK T { PRIORITY = FULL; SCHEDULE = FULL; APPLICATION = A; };").unwrap_err();
        assert!(err.message.contains("PRIORITY must be a number"));
    }

    #[test]
    fn application_must_be_an_identifier() {
        let err =
            parse("TASK T { PRIORITY = 1; SCHEDULE = FULL; APPLICATION = 42; };").unwrap_err();
        assert!(err.message.contains("APPLICATION must be an identifier"));
    }

    /// Regression test: the lexer used to silently clamp an overflowing
    /// digit run to `u32::MAX` and hand the parser a *valid*-looking
    /// `Token::Number`, so a `.cfg` with e.g. `PRIORITY = 99999999999;`
    /// would parse successfully with a wrong value instead of failing.
    /// Fixed by emitting overflowing digit runs as `Token::Ident` instead,
    /// which correctly fails value-type checking.
    #[test]
    fn rejects_priority_that_overflows_u32() {
        let err = parse("TASK T { PRIORITY = 99999999999; SCHEDULE = FULL; APPLICATION = A; };")
            .unwrap_err();
        assert!(err.message.contains("PRIORITY must be a number"));
    }

    #[test]
    fn tabs_and_crlf_are_whitespace() {
        let src =
            "TASK\tT\t{\r\n\tPRIORITY\t=\t1;\r\n\tSCHEDULE = FULL;\r\n\tAPPLICATION = A;\r\n};";
        let config = parse(src).unwrap();
        assert_eq!(config.tasks[0].name, "T");
    }
}
