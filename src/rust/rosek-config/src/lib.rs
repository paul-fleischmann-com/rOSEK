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
