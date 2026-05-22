use std::iter::Peekable;

use crate::ast::{
    Bar, BarElement, Duration, Grace, Header, Key, Mode, Pitch, Section, TimeSignature, Token, Tune,
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
        let (sections, final_bar) = self.parse_sections();
        Ok(Tune { header, sections, final_bar })
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

    #[allow(unused_assignments)]
    fn parse_sections(&mut self) -> (Vec<Section>, bool) {
        let mut sections: Vec<Section> = Vec::new();
        let mut final_bar = false;

        // Accumulator for the bar currently being built
        let mut cur: Vec<BarElement> = Vec::new();

        // Where completed bars are deposited — depends on which phase we're in
        let mut plain:   Vec<Bar> = Vec::new();   // before any repeat
        let mut body:    Vec<Bar> = Vec::new();   // inside a repeat, before first volta
        let mut alts:    Vec<Vec<Bar>> = Vec::new(); // completed volta alternatives
        let mut cur_alt: Vec<Bar> = Vec::new();   // the volta alternative being built

        let mut in_repeat = false;
        let mut in_alt    = false;

        macro_rules! flush_bar {
            () => {
                if !cur.is_empty() {
                    let bar = Bar { elements: std::mem::take(&mut cur) };
                    if in_alt        { cur_alt.push(bar); }
                    else if in_repeat { body.push(bar); }
                    else              { plain.push(bar); }
                }
            };
        }

        macro_rules! finish_repeat {
            () => {
                sections.push(Section::Repeat {
                    body: std::mem::take(&mut body),
                    alternatives: std::mem::take(&mut alts),
                });
                in_repeat = false;
                in_alt    = false;
            };
        }

        loop {
            match self.tokens.next() {
                None => break,

                Some(Token::Note(n)) => cur.push(BarElement::Note(n)),

                Some(Token::Grace(grace_notes, acciaccatura)) => {
                    if let Some(Token::Note(mut main)) = self.tokens.next() {
                        main.grace = Some(Grace { notes: grace_notes, acciaccatura });
                        cur.push(BarElement::Note(main));
                    }
                }

                Some(Token::Tuplet(t)) => {
                    let count = t.r.unwrap_or(t.p) as usize;
                    let notes = (0..count)
                        .filter_map(|_| match self.tokens.next() {
                            Some(Token::Note(n)) => Some(n),
                            _ => None,
                        })
                        .collect();
                    cur.push(BarElement::Tuplet(t, notes));
                }

                Some(Token::Bar) => { flush_bar!(); }

                Some(Token::RepeatStart) => {
                    flush_bar!();
                    if !plain.is_empty() {
                        sections.push(Section::Plain(std::mem::take(&mut plain)));
                    }
                    in_repeat = true;
                }

                Some(Token::Volta(_)) => {
                    flush_bar!();
                    in_alt = true;
                }

                Some(Token::RepeatEnd) => {
                    flush_bar!();
                    if in_alt {
                        alts.push(std::mem::take(&mut cur_alt));
                        // If a Volta token follows, another alternative is starting
                        if matches!(self.tokens.peek(), Some(Token::Volta(_))) {
                            // Stay in alt mode; cur_alt already empty
                        } else {
                            finish_repeat!();
                        }
                    } else {
                        // :|  without a preceding |: — treat accumulated plain bars
                        // as the repeat body (common in ABC tunes from thesession etc.)
                        if !plain.is_empty() {
                            body = std::mem::take(&mut plain);
                        }
                        finish_repeat!();
                    }
                }

                Some(Token::FinalBar) => {
                    flush_bar!();
                    final_bar = true;
                    if in_alt {
                        alts.push(std::mem::take(&mut cur_alt));
                        finish_repeat!();
                    } else if in_repeat {
                        finish_repeat!();
                    } else if !plain.is_empty() {
                        sections.push(Section::Plain(std::mem::take(&mut plain)));
                    }
                }

                Some(Token::DoubleBar) => {
                    flush_bar!();
                    if in_alt {
                        alts.push(std::mem::take(&mut cur_alt));
                        finish_repeat!();
                    } else if in_repeat {
                        finish_repeat!();
                    } else if !plain.is_empty() {
                        sections.push(Section::Plain(std::mem::take(&mut plain)));
                    }
                    sections.push(Section::DoubleBar);
                }

                Some(Token::RepeatEndStart) => {
                    // :|: — end a repeat and immediately start a new one
                    flush_bar!();
                    if in_alt {
                        alts.push(std::mem::take(&mut cur_alt));
                    }
                    finish_repeat!();
                    in_repeat = true;
                }

                Some(Token::Header(_, _) | Token::Unknown) => {}
            }
        }

        // Flush any trailing material
        flush_bar!();
        if in_alt {
            alts.push(std::mem::take(&mut cur_alt));
            finish_repeat!();
        } else if in_repeat && !body.is_empty() {
            finish_repeat!();
        } else if !plain.is_empty() {
            sections.push(Section::Plain(std::mem::take(&mut plain)));
        }

        (sections, final_bar)
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
        'C' => Pitch::C, 'D' => Pitch::D, 'E' => Pitch::E, 'F' => Pitch::F,
        'G' => Pitch::G, 'A' => Pitch::A, 'B' => Pitch::B,
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

    fn plain_bars(tune: &Tune) -> &Vec<Bar> {
        match &tune.sections[0] {
            Section::Plain(bars) => bars,
            s => panic!("expected Section::Plain, got {s:?}"),
        }
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
        let bars = plain_bars(&tune);
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].elements.len(), 3);
        assert_eq!(bars[1].elements.len(), 3);
    }

    #[test]
    fn trailing_notes_become_final_bar() {
        let tune = parse("M:4/4\nL:1/8\nK:D\nabc").unwrap();
        let bars = plain_bars(&tune);
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].elements.len(), 3);
    }

    #[test]
    fn parses_note_fields() {
        let tune = parse("M:4/4\nL:1/8\nK:D\n^c'2").unwrap();
        let bars = plain_bars(&tune);
        let BarElement::Note(n) = &bars[0].elements[0] else { panic!("expected note") };
        assert!(matches!(n.pitch, Pitch::C));
        assert!(matches!(n.accidental, Some(Accidental::Sharp)));
        assert_eq!(n.octave, 1);
        assert_eq!(n.duration.numerator, 2);
    }

    #[test]
    fn parses_tuplet() {
        let tune = parse("M:4/4\nL:1/8\nK:D\n(3cde").unwrap();
        let bars = plain_bars(&tune);
        let BarElement::Tuplet(t, notes) = &bars[0].elements[0] else { panic!("expected tuplet") };
        assert_eq!(t.p, 3);
        assert_eq!(notes.len(), 3);
    }

    #[test]
    fn tuplet_with_explicit_r() {
        let tune = parse("M:4/4\nL:1/8\nK:D\n(3:2:3cde").unwrap();
        let bars = plain_bars(&tune);
        let BarElement::Tuplet(t, notes) = &bars[0].elements[0] else { panic!("expected tuplet") };
        assert_eq!(t.r, Some(3));
        assert_eq!(notes.len(), 3);
    }

    #[test]
    fn parses_repeat_with_alternatives() {
        let tune = parse("M:4/4\nL:1/8\nK:D\n|:abc|def|1gab:|2gcd||").unwrap();
        let Section::Repeat { body, alternatives } = &tune.sections[0] else {
            panic!("expected Repeat section");
        };
        assert_eq!(body.len(), 2);       // abc, def
        assert_eq!(alternatives.len(), 2); // gab, gcd
        assert!(matches!(tune.sections[1], Section::DoubleBar));
    }

    #[test]
    fn parses_two_repeat_sections() {
        let tune = parse("M:4/4\nL:1/8\nK:D\n|:abc:||\n|:def:||\n").unwrap();
        assert_eq!(tune.sections.len(), 2);
        assert!(matches!(tune.sections[0], Section::Repeat { .. }));
        assert!(matches!(tune.sections[1], Section::Repeat { .. }));
    }

    #[test]
    fn repeat_end_without_start_uses_plain_bars_as_body() {
        // Second section has no |: — bars before :| should become the repeat body
        let tune = parse("M:4/4\nL:1/8\nK:D\n|:abc:|def:|").unwrap();
        assert_eq!(tune.sections.len(), 2);
        let Section::Repeat { body, .. } = &tune.sections[1] else {
            panic!("expected Repeat");
        };
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].elements.len(), 3); // d e f
    }
}
