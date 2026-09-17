//! Tokeniser for the config grammar (see crate docs). Skips whitespace and
//! `//` line comments, tracks line numbers for [`crate::ParseError`].

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token<'a> {
    Ident(&'a str),
    Number(u32),
    LBrace,
    RBrace,
    Equals,
    Semicolon,
}

pub(crate) struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    line: usize,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(src: &'a str) -> Self {
        Lexer {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            line: 1,
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek_byte()?;
        self.pos += 1;
        if b == b'\n' {
            self.line += 1;
        }
        Some(b)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek_byte() {
                Some(b) if b.is_ascii_whitespace() => {
                    self.bump();
                }
                Some(b'/') if self.bytes.get(self.pos + 1) == Some(&b'/') => {
                    while let Some(b) = self.peek_byte() {
                        if b == b'\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                _ => break,
            }
        }
    }

    fn is_ident_start(b: u8) -> bool {
        b.is_ascii_alphabetic() || b == b'_'
    }

    fn is_ident_continue(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_'
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = (Token<'a>, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace_and_comments();
        let start = self.pos;
        let line = self.line;
        let b = self.peek_byte()?;

        let token = match b {
            b'{' => {
                self.bump();
                Token::LBrace
            }
            b'}' => {
                self.bump();
                Token::RBrace
            }
            b'=' => {
                self.bump();
                Token::Equals
            }
            b';' => {
                self.bump();
                Token::Semicolon
            }
            b if b.is_ascii_digit() => {
                while self.peek_byte().map_or(false, |b| b.is_ascii_digit()) {
                    self.bump();
                }
                let text = &self.src[start..self.pos];
                // A digit run that doesn't fit u32 (overflow) is emitted as
                // an Ident instead of silently clamping to u32::MAX, which
                // would make the parser accept a wrong value without any
                // diagnostic. As an Ident, it correctly fails wherever a
                // Number was required ("PRIORITY must be a number") instead
                // of being silently wrong.
                match text.parse::<u32>() {
                    Ok(n) => Token::Number(n),
                    Err(_) => Token::Ident(text),
                }
            }
            b if Self::is_ident_start(b) => {
                self.bump();
                while self.peek_byte().map_or(false, Self::is_ident_continue) {
                    self.bump();
                }
                Token::Ident(&self.src[start..self.pos])
            }
            _ => {
                // Unrecognised character: consume it and let it show up as
                // an empty/garbled identifier the parser rejects, keeping
                // the lexer infallible (no separate lexer-error type).
                self.bump();
                Token::Ident(&self.src[start..self.pos])
            }
        };

        Some((token, line))
    }
}
