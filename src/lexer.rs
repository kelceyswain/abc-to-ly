use crate::ast::{Token, Accidental, Ornament, Pitch, Note, Duration, Tuplet};

use std::str::Chars;
use std::iter::Peekable;

pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    at_line_start: bool,
    push_back: Option<Token>,
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(t) = self.push_back.take() {
            return Some(t);
        }
        self.next_token()
    }
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { chars: input.chars().peekable(), at_line_start: true, push_back: None }
    }

    fn next_token(&mut self) -> Option<Token> {
        loop {
            while let Some(&ch) = self.chars.peek() {
                match ch {
                    ' ' | '\r' | '\t' => { self.chars.next(); }
                    '\n' => { self.chars.next(); self.at_line_start = true; }
                    _ => break,
                }
            }

            if self.at_line_start {
                if let Some(&c) = self.chars.peek() {
                    if c.is_ascii_alphabetic() {
                        self.at_line_start = false;
                        let key = self.chars.next().unwrap();
                        if self.chars.peek().copied() == Some(':') {
                            self.chars.next(); // consume ':'
                            return self.lex_header_rest(key);
                        } else {
                            return self.lex_note_rest(key, None, None);
                        }
                    }
                }
            }

            self.at_line_start = false;

            return match self.chars.peek()? {
                'A'..='G' | 'a'..='g' => self.lex_note(),
                '^' | '_' | '=' | '~' => self.lex_note(),
                '{' => self.lex_grace(),
                '(' => self.lex_tuplet(),
                '|' => self.lex_barline(),
                ':' => self.lex_repeat_end(),
                '%' => { self.skip_line(); self.at_line_start = true; continue; }
                _ => { self.chars.next(); Some(Token::Unknown) }
            };
        }
    }

    fn lex_barline(&mut self) -> Option<Token> {
        self.chars.next(); // consume '|'
        match self.chars.peek().copied() {
            Some(':') => { self.chars.next(); Some(Token::RepeatStart) }
            Some('|') => { self.chars.next(); Some(Token::DoubleBar) }
            Some(']') => { self.chars.next(); Some(Token::FinalBar) }
            Some(c) if c.is_ascii_digit() => {
                self.chars.next();
                Some(Token::Volta(c as u8 - b'0'))
            }
            _ => Some(Token::Bar)
        }
    }

    fn lex_repeat_end(&mut self) -> Option<Token> {
        self.chars.next(); // consume ':'
        match self.chars.peek().copied() {
            Some('|') => {
                self.chars.next();
                // :|n — push volta token so it's returned next call
                if let Some(c) = self.chars.peek().copied() {
                    if c.is_ascii_digit() {
                        self.chars.next();
                        self.push_back = Some(Token::Volta(c as u8 - b'0'));
                    }
                }
                Some(Token::RepeatEnd)
            }
            Some(':') => { self.chars.next(); Some(Token::RepeatEndStart) }
            _ => Some(Token::Unknown)
        }
    }

    fn lex_note(&mut self) -> Option<Token> {
        let ornament = if self.chars.peek() == Some(&'~') {
            self.chars.next();
            Some(Ornament::Turn)
        } else {
            None
        };
        let accidental = match self.chars.peek() {
            Some('^') => { self.chars.next(); Some(Accidental::Sharp) }
            Some('_') => { self.chars.next(); Some(Accidental::Flat) }
            Some('=') => { self.chars.next(); Some(Accidental::Natural) }
            _ => None,
        };
        let pitch_char = self.chars.next()?;
        self.lex_note_rest(pitch_char, accidental, ornament)
    }

    fn lex_note_rest(&mut self, pitch_char: char, accidental: Option<Accidental>, ornament: Option<Ornament>) -> Option<Token> {
        let (pitch, base_octave) = match pitch_char {
            'C' => (Pitch::C, -1),
            'D' => (Pitch::D, -1),
            'E' => (Pitch::E, -1),
            'F' => (Pitch::F, -1),
            'G' => (Pitch::G, -1),
            'A' => (Pitch::A, -1),
            'B' => (Pitch::B, -1),
            'c' => (Pitch::C, 0),
            'd' => (Pitch::D, 0),
            'e' => (Pitch::E, 0),
            'f' => (Pitch::F, 0),
            'g' => (Pitch::G, 0),
            'a' => (Pitch::A, 0),
            'b' => (Pitch::B, 0),
            _ => return None,
        };

        let mut octave = base_octave;
        while let Some(&c) = self.chars.peek() {
            match c {
                '\'' => { octave += 1; self.chars.next(); }
                ',' => { octave -= 1; self.chars.next(); }
                _ => break,
            }
        }

        let duration = self.lex_duration();
        Some(Token::Note(Note { pitch, octave, accidental, ornament, grace: None, duration }))
    }

    fn lex_duration(&mut self) -> Duration {
        // optional numerator
        let numerator = self.lex_digits().unwrap_or(1);

        // optional slash(es)
        let denominator = if let Some('/') = self.chars.peek().copied() {
            self.chars.next(); // consume first '/'

            // count any additinoal slashes
            let mut denom = 2u8;
            while let Some('/') = self.chars.peek().copied() {
                self.chars.next();
                denom *= 2;
            }

            // explicit denominator after slash overrides the slash-count
            self.lex_digits().unwrap_or(denom)
        } else {
            1
        };

        Duration { numerator, denominator }
    }

    fn lex_digits(&mut self) -> Option<u8> {
        let mut s = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.chars.next();
            } else {
                break;
            }
        }
        s.parse().ok()
    }

    fn lex_tuplet(&mut self) -> Option<Token> {
        self.chars.next(); // consume '('
        let p = self.lex_digits()?;
        let q = if self.chars.peek().copied() == Some(':') {
            self.chars.next();
            self.lex_digits()
        } else {
            None
        };
        let r = if q.is_some() && self.chars.peek().copied() == Some(':') {
            self.chars.next();
            self.lex_digits()
        } else {
            None
        };
        Some(Token::Tuplet(Tuplet { p, q, r }))
    }

    fn lex_grace(&mut self) -> Option<Token> {
        self.chars.next(); // consume '{'
        let acciaccatura = if self.chars.peek() == Some(&'/') {
            self.chars.next();
            true
        } else {
            false
        };

        let mut notes = Vec::new();
        while let Some(&c) = self.chars.peek() {
            match c {
                '}' => { self.chars.next(); break; }
                ' ' => { self.chars.next(); }
                _ => {
                    let accidental = match self.chars.peek() {
                        Some('^') => { self.chars.next(); Some(Accidental::Sharp) }
                        Some('_') => { self.chars.next(); Some(Accidental::Flat) }
                        Some('=') => { self.chars.next(); Some(Accidental::Natural) }
                        _ => None,
                    };
                    let Some(&pc) = self.chars.peek() else { break };
                    if !pc.is_ascii_alphabetic() { self.chars.next(); continue; }
                    self.chars.next();
                    let base_octave: i8 = if pc.is_uppercase() { -1 } else { 0 };
                    let mut octave = base_octave;
                    while let Some(&m) = self.chars.peek() {
                        match m {
                            '\'' => { octave += 1; self.chars.next(); }
                            ',' => { octave -= 1; self.chars.next(); }
                            _ => break,
                        }
                    }
                    let pitch = match pc.to_ascii_lowercase() {
                        'c' => Pitch::C, 'd' => Pitch::D, 'e' => Pitch::E,
                        'f' => Pitch::F, 'g' => Pitch::G, 'a' => Pitch::A,
                        'b' => Pitch::B, _ => continue,
                    };
                    notes.push(Note {
                        pitch, octave, accidental,
                        ornament: None, grace: None,
                        duration: Duration { numerator: 1, denominator: 1 },
                    });
                }
            }
        }

        if notes.is_empty() { None } else { Some(Token::Grace(notes, acciaccatura)) }
    }

    fn lex_header_rest(&mut self, key: char) -> Option<Token> {
        let mut value = String::new();
        while let Some(&c) = self.chars.peek() {
            if c == '\n' { break; }
            value.push(c);
            self.chars.next();
        }
        Some(Token::Header(key, value.trim().to_string()))
    }

    fn skip_line(&mut self) {
        while let Some(&c) = self.chars.peek() {
            self.chars.next();
            if c == '\n' { break; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Accidental, Pitch, Token};

    fn lex(input: &str) -> Vec<Token> {
        Lexer::new(input).collect()
    }

    fn get_note(tokens: &[Token], i: usize) -> &Note {
        match &tokens[i] {
            Token::Note(n) => n,
            t => panic!("expected Note at index {i}, got {t:?}"),
        }
    }

    // --- headers ---

    #[test]
    fn header_single() {
        let tokens = lex("T:My Song");
        assert_eq!(tokens.len(), 1);
        assert!(matches!(&tokens[0], Token::Header('T', s) if s == "My Song"));
    }

    #[test]
    fn header_trims_whitespace() {
        let tokens = lex("T:  My Song  ");
        assert!(matches!(&tokens[0], Token::Header('T', s) if s == "My Song"));
    }

    #[test]
    fn header_multiple() {
        let tokens = lex("T:My Song\nM:4/4\nL:1/8\nK:G");
        assert_eq!(tokens.len(), 4);
        assert!(matches!(&tokens[0], Token::Header('T', s) if s == "My Song"));
        assert!(matches!(&tokens[1], Token::Header('M', s) if s == "4/4"));
        assert!(matches!(&tokens[2], Token::Header('L', s) if s == "1/8"));
        assert!(matches!(&tokens[3], Token::Header('K', s) if s == "G"));
    }

    // --- notes: pitch and octave ---

    #[test]
    fn note_lowercase_middle_octave() {
        let tokens = lex("c");
        let n = get_note(&tokens, 0);
        assert!(matches!(n.pitch, Pitch::C));
        assert_eq!(n.octave, 0);
    }

    #[test]
    fn note_uppercase_low_octave() {
        let tokens = lex("C");
        let n = get_note(&tokens, 0);
        assert!(matches!(n.pitch, Pitch::C));
        assert_eq!(n.octave, -1);
    }

    #[test]
    fn note_octave_up() {
        let tokens = lex("c'");
        let n = get_note(&tokens, 0);
        assert_eq!(n.octave, 1);
    }

    #[test]
    fn note_octave_up_twice() {
        let tokens = lex("c''");
        let n = get_note(&tokens, 0);
        assert_eq!(n.octave, 2);
    }

    #[test]
    fn note_octave_down() {
        let tokens = lex("C,");
        let n = get_note(&tokens, 0);
        assert_eq!(n.octave, -2);
    }

    // --- notes: accidentals ---

    #[test]
    fn note_sharp() {
        let tokens = lex("^c");
        let n = get_note(&tokens, 0);
        assert!(matches!(n.accidental, Some(Accidental::Sharp)));
        assert!(matches!(n.pitch, Pitch::C));
    }

    #[test]
    fn note_flat() {
        let tokens = lex("_b");
        let n = get_note(&tokens, 0);
        assert!(matches!(n.accidental, Some(Accidental::Flat)));
    }

    #[test]
    fn note_natural() {
        let tokens = lex("=f");
        let n = get_note(&tokens, 0);
        assert!(matches!(n.accidental, Some(Accidental::Natural)));
    }

    #[test]
    fn note_no_accidental() {
        let tokens = lex("d");
        let n = get_note(&tokens, 0);
        assert!(n.accidental.is_none());
    }

    // --- notes: duration ---

    #[test]
    fn note_default_duration() {
        let tokens = lex("c");
        let n = get_note(&tokens, 0);
        assert_eq!(n.duration.numerator, 1);
        assert_eq!(n.duration.denominator, 1);
    }

    #[test]
    fn note_duration_double() {
        let tokens = lex("c2");
        let n = get_note(&tokens, 0);
        assert_eq!(n.duration.numerator, 2);
        assert_eq!(n.duration.denominator, 1);
    }

    #[test]
    fn note_duration_half_explicit() {
        let tokens = lex("c/2");
        let n = get_note(&tokens, 0);
        assert_eq!(n.duration.numerator, 1);
        assert_eq!(n.duration.denominator, 2);
    }

    #[test]
    fn note_duration_half_bare_slash() {
        let tokens = lex("c/");
        let n = get_note(&tokens, 0);
        assert_eq!(n.duration.numerator, 1);
        assert_eq!(n.duration.denominator, 2);
    }

    #[test]
    fn note_duration_double_slash() {
        let tokens = lex("c//");
        let n = get_note(&tokens, 0);
        assert_eq!(n.duration.numerator, 1);
        assert_eq!(n.duration.denominator, 4);
    }

    #[test]
    fn note_duration_dotted() {
        let tokens = lex("c3/2");
        let n = get_note(&tokens, 0);
        assert_eq!(n.duration.numerator, 3);
        assert_eq!(n.duration.denominator, 2);
    }

    // --- barlines ---

    #[test]
    fn barline_simple() {
        let tokens = lex("c | d");
        assert!(matches!(tokens[1], Token::Bar));
    }

    #[test]
    fn barline_double() {
        let tokens = lex("c || d");
        assert!(matches!(tokens[1], Token::DoubleBar));
    }

    #[test]
    fn barline_repeat_start() {
        let tokens = lex("c |: d");
        assert!(matches!(tokens[1], Token::RepeatStart));
    }

    #[test]
    fn barline_repeat_end() {
        let tokens = lex("c :| d");
        assert!(matches!(tokens[1], Token::RepeatEnd));
    }

    // --- comments ---

    #[test]
    fn comment_skips_rest_of_line() {
        let tokens = lex("c % ignored\nd");
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0], Token::Note(_)));
        assert!(matches!(tokens[1], Token::Note(_)));
    }

    // --- tuplets ---

    #[test]
    fn tuplet_simple_triplet() {
        let tokens = lex("(3cde");
        assert!(matches!(&tokens[0], Token::Tuplet(t) if t.p == 3 && t.q.is_none() && t.r.is_none()));
        assert_eq!(tokens.len(), 4);
    }

    #[test]
    fn tuplet_with_q() {
        let tokens = lex("(3:2cde");
        assert!(matches!(&tokens[0], Token::Tuplet(t) if t.p == 3 && t.q == Some(2) && t.r.is_none()));
    }

    #[test]
    fn tuplet_full_form() {
        let tokens = lex("(3:2:3cde");
        assert!(matches!(&tokens[0], Token::Tuplet(t) if t.p == 3 && t.q == Some(2) && t.r == Some(3)));
    }

    #[test]
    fn tuplet_duplet() {
        let tokens = lex("(2cd");
        assert!(matches!(&tokens[0], Token::Tuplet(t) if t.p == 2));
        assert_eq!(tokens.len(), 3);
    }

    // --- ornaments ---

    #[test]
    fn turn_ornament_on_uppercase() {
        let tokens = lex("~G");
        let n = get_note(&tokens, 0);
        assert!(matches!(n.pitch, Pitch::G));
        assert!(matches!(n.ornament, Some(crate::ast::Ornament::Turn)));
    }

    #[test]
    fn turn_ornament_with_accidental() {
        let tokens = lex("~^g");
        let n = get_note(&tokens, 0);
        assert!(matches!(n.accidental, Some(Accidental::Sharp)));
        assert!(matches!(n.ornament, Some(crate::ast::Ornament::Turn)));
    }

    #[test]
    fn no_ornament_on_plain_note() {
        let tokens = lex("g");
        let n = get_note(&tokens, 0);
        assert!(n.ornament.is_none());
    }

    // --- regression: notes at line start ---

    #[test]
    fn notes_at_line_start_not_parsed_as_header() {
        let tokens = lex("T:My Song\nabc");
        assert_eq!(tokens.len(), 4);
        assert!(matches!(&tokens[0], Token::Header('T', _)));
        assert!(matches!(tokens[1], Token::Note(_)));
        assert!(matches!(tokens[2], Token::Note(_)));
        assert!(matches!(tokens[3], Token::Note(_)));
    }

    // --- full tune ---

    #[test]
    fn full_tune() {
        let tokens = lex("T:The Morning Dew\nM:4/4\nL:1/8\nK:D\nabc ABC | def DEF");
        let headers = tokens.iter().filter(|t| matches!(t, Token::Header(_, _))).count();
        let notes   = tokens.iter().filter(|t| matches!(t, Token::Note(_))).count();
        let bars    = tokens.iter().filter(|t| matches!(t, Token::Bar)).count();
        assert_eq!(headers, 4);
        assert_eq!(notes, 12);
        assert_eq!(bars, 1);
    }
}