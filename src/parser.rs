use std::iter::Peekable;

use crate::ast::{
    Bar, BarElement, Duration, Header, Key, Mode, Pitch, TimeSignature, Token, Tune,
};

#[allow(dead_code)]
#[derive(Debug)]
pub enum ParseError {
    MissingHeader(char),
    InvalidValue(char, String),
}

pub struct Parser<I: Iterator<Item = Token>> {
    tokens: Peekable<I>,
}

impl<I: Iterator<Item = Token>> Parser<I> {
    pub fn new(tokens: I) -> Self {
        Self { tokens: tokens.peekable() }
    }

    pub fn parse(&mut self) -> Result<Tune, ParseError> {
        let header = self.parse_header()?;
        let bars = self.parse_body();
        Ok(Tune { header, bars })
    }

    fn parse_header(&mut self) -> Result<Header, ParseError> {
        let mut title = String::new();
        let mut time: Option<TimeSignature> = None;
        let mut default_length: Option<Duration> = None;
        let mut key: Option<Key> = None;

        while matches!(self.tokens.peek(), Some(Token::Header(_, _))) {
            let Some(Token::Header(k, v)) = self.tokens.next() else { break };
            match k {
                'T' => title = v,
                'M' => time = Some(parse_time_sig(&v).ok_or(ParseError::InvalidValue('M', v))?),
                'L' => default_length = Some(parse_fraction(&v).ok_or(ParseError::InvalidValue('L', v))?),
                'K' => key = Some(parse_key(&v).ok_or(ParseError::InvalidValue('K', v))?),
                _ => {}
            }
        }

        Ok(Header {
            title,
            key: key.ok_or(ParseError::MissingHeader('K'))?,
            time: time.ok_or(ParseError::MissingHeader('M'))?,
            default_length: default_length.ok_or(ParseError::MissingHeader('L'))?,
        })
    }

    fn parse_body(&mut self) -> Vec<Bar> {
        let mut bars: Vec<Bar> = Vec::new();
        let mut current: Vec<BarElement> = Vec::new();

        loop {
            match self.tokens.next() {
                None => break,
                Some(Token::Note(n)) => current.push(BarElement::Note(n)),
                Some(Token::Tuplet(t)) => {
                    let count = t.r.unwrap_or(t.p) as usize;
                    let notes = (0..count)
                        .filter_map(|_| match self.tokens.next() {
                            Some(Token::Note(n)) => Some(n),
                            _ => None,
                        })
                        .collect();
                    current.push(BarElement::Tuplet(t, notes));
                }
                Some(Token::Bar | Token::DoubleBar | Token::RepeatStart
                    | Token::RepeatEnd | Token::RepeatEndStart) => {
                    if !current.is_empty() {
                        bars.push(Bar { elements: current });
                        current = Vec::new();
                    }
                }
                Some(Token::Header(_, _) | Token::Unknown) => {}
            }
        }

        if !current.is_empty() {
            bars.push(Bar { elements: current });
        }

        bars
    }
}

fn parse_fraction(s: &str) -> Option<Duration> {
    let (num, den) = s.split_once('/')?;
    Some(Duration {
        numerator: num.trim().parse().ok()?,
        denominator: den.trim().parse().ok()?,
    })
}

fn parse_time_sig(s: &str) -> Option<TimeSignature> {
    let (num, den) = s.split_once('/')?;
    Some(TimeSignature {
        numerator: num.trim().parse().ok()?,
        denominator: den.trim().parse().ok()?,
    })
}

fn parse_key(s: &str) -> Option<Key> {
    let mut chars = s.chars();
    let pitch = match chars.next()? {
        'C' => Pitch::C,
        'D' => Pitch::D,
        'E' => Pitch::E,
        'F' => Pitch::F,
        'G' => Pitch::G,
        'A' => Pitch::A,
        'B' => Pitch::B,
        _ => return None,
    };
    let mode = match chars.as_str().to_lowercase().as_str() {
        "" | "maj" | "major" => Mode::Major,
        "m" | "min" | "minor" => Mode::Minor,
        "dor" | "dorian" => Mode::Dorian,
        "mix" | "mixolydian" => Mode::Mixolydian,
        _ => return None,
    };
    Some(Key { pitch, mode })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Accidental, Pitch};
    use crate::lexer::Lexer;

    fn parse(input: &str) -> Result<Tune, ParseError> {
        Parser::new(Lexer::new(input)).parse()
    }

    #[test]
    fn parses_header_fields() {
        let tune = parse("T:The Morning Dew\nM:4/4\nL:1/8\nK:D").unwrap();
        assert_eq!(tune.header.title, "The Morning Dew");
        assert_eq!(tune.header.time.numerator, 4);
        assert_eq!(tune.header.time.denominator, 4);
        assert_eq!(tune.header.default_length.numerator, 1);
        assert_eq!(tune.header.default_length.denominator, 8);
        assert!(matches!(tune.header.key.pitch, Pitch::D));
        assert!(matches!(tune.header.key.mode, Mode::Major));
    }

    #[test]
    fn parses_minor_key() {
        let tune = parse("M:4/4\nL:1/8\nK:Am").unwrap();
        assert!(matches!(tune.header.key.pitch, Pitch::A));
        assert!(matches!(tune.header.key.mode, Mode::Minor));
    }

    #[test]
    fn parses_dorian_mode() {
        let tune = parse("M:4/4\nL:1/8\nK:Ddor").unwrap();
        assert!(matches!(tune.header.key.mode, Mode::Dorian));
    }

    #[test]
    fn missing_key_is_error() {
        let result = parse("T:Song\nM:4/4\nL:1/8");
        assert!(matches!(result, Err(ParseError::MissingHeader('K'))));
    }

    #[test]
    fn missing_time_is_error() {
        let result = parse("T:Song\nL:1/8\nK:D");
        assert!(matches!(result, Err(ParseError::MissingHeader('M'))));
    }

    #[test]
    fn invalid_time_sig_is_error() {
        let result = parse("M:4x4\nL:1/8\nK:D");
        assert!(matches!(result, Err(ParseError::InvalidValue('M', _))));
    }

    #[test]
    fn groups_notes_into_bars() {
        let tune = parse("M:4/4\nL:1/8\nK:D\nabc | def").unwrap();
        assert_eq!(tune.bars.len(), 2);
        assert_eq!(tune.bars[0].elements.len(), 3);
        assert_eq!(tune.bars[1].elements.len(), 3);
    }

    #[test]
    fn trailing_notes_become_final_bar() {
        let tune = parse("M:4/4\nL:1/8\nK:D\nabc").unwrap();
        assert_eq!(tune.bars.len(), 1);
        assert_eq!(tune.bars[0].elements.len(), 3);
    }

    #[test]
    fn parses_note_fields() {
        let tune = parse("M:4/4\nL:1/8\nK:D\n^c'2").unwrap();
        let BarElement::Note(n) = &tune.bars[0].elements[0] else { panic!("expected note") };
        assert!(matches!(n.pitch, Pitch::C));
        assert!(matches!(n.accidental, Some(Accidental::Sharp)));
        assert_eq!(n.octave, 1);
        assert_eq!(n.duration.numerator, 2);
    }

    #[test]
    fn parses_tuplet() {
        let tune = parse("M:4/4\nL:1/8\nK:D\n(3cde").unwrap();
        let BarElement::Tuplet(t, notes) = &tune.bars[0].elements[0] else { panic!("expected tuplet") };
        assert_eq!(t.p, 3);
        assert_eq!(notes.len(), 3);
    }

    #[test]
    fn tuplet_with_explicit_r() {
        let tune = parse("M:4/4\nL:1/8\nK:D\n(3:2:3cde").unwrap();
        let BarElement::Tuplet(t, notes) = &tune.bars[0].elements[0] else { panic!("expected tuplet") };
        assert_eq!(t.r, Some(3));
        assert_eq!(notes.len(), 3);
    }
}
