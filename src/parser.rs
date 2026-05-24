use std::iter::Peekable;

use crate::ast::{
    Bar, BarElement, Duration, Grace, Header, Key, Mode, Pitch, Section, Tempo,
    TimeSignature, TimeSymbol, Token, Tune,
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
        let mut tempo: Option<Tempo> = None;

        while matches!(self.tokens.peek(), Some(Token::Header(_, _))) {
            let Some(Token::Header(k, v)) = self.tokens.next() else { break };
            match k {
                'T' => title = v,
                'M' => time = Some(parse_time_sig(&v).ok_or(ParseError::InvalidValue('M', v))?),
                'L' => default_length = Some(parse_fraction(&v).ok_or(ParseError::InvalidValue('L', v))?),
                'K' => key = Some(parse_key(&v).ok_or(ParseError::InvalidValue('K', v))?),
                'Q' => tempo = parse_tempo(&v), // soft failure — bad Q: is silently ignored
                _ => {}
            }
        }

        Ok(Header {
            title,
            key: key.ok_or(ParseError::MissingHeader('K'))?,
            time: time.ok_or(ParseError::MissingHeader('M'))?,
            default_length: default_length.ok_or(ParseError::MissingHeader('L'))?,
            tempo,
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

                Some(Token::Rest(r)) => {
                    cur.push(BarElement::Rest(r));
                }

                Some(Token::Note(mut n)) => {
                    // Count consecutive broken rhythm arrows; stop if direction flips
                    let mut count: i32 = 0;
                    loop {
                        match self.tokens.peek() {
                            Some(Token::BrokenRight) if count >= 0 => { self.tokens.next(); count += 1; }
                            Some(Token::BrokenLeft)  if count <= 0 => { self.tokens.next(); count -= 1; }
                            _ => break,
                        }
                    }
                    if count != 0 {
                        let arrows = count.unsigned_abs();
                        n.duration = broken_dur(n.duration, count > 0, arrows);
                        cur.push(BarElement::Note(n));
                        if let Some(Token::Note(mut m)) = self.tokens.next() {
                            m.duration = broken_dur(m.duration, count < 0, arrows);
                            cur.push(BarElement::Note(m));
                        }
                    } else {
                        cur.push(BarElement::Note(n));
                    }
                }

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

                Some(Token::Header(_, _) | Token::Unknown
                    | Token::BrokenRight | Token::BrokenLeft) => {}
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

// Long note (n arrows): duration × (2^(n+1) − 1) / 2^n
// Short note:            duration × 1 / 2^n
fn broken_dur(dur: Duration, long: bool, n: u32) -> Duration {
    let factor = 1u32 << n;
    let (new_num, new_den) = if long {
        (dur.numerator as u32 * (factor * 2 - 1), dur.denominator as u32 * factor)
    } else {
        (dur.numerator as u32, dur.denominator as u32 * factor)
    };
    let g = gcd(new_num, new_den);
    Duration { numerator: (new_num / g) as u8, denominator: (new_den / g) as u8 }
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn parse_fraction(s: &str) -> Option<Duration> {
    let (num, den) = s.split_once('/')?;
    Some(Duration {
        numerator: num.trim().parse().ok()?,
        denominator: den.trim().parse().ok()?,
    })
}

fn parse_time_sig(s: &str) -> Option<TimeSignature> {
    let s = s.trim();
    match s {
        "C"  => return Some(TimeSignature { numerator: 4, denominator: 4, symbol: Some(TimeSymbol::Common) }),
        "C|" => return Some(TimeSignature { numerator: 2, denominator: 2, symbol: Some(TimeSymbol::Cut) }),
        _ => {}
    }
    let (num, den) = s.split_once('/')?;
    Some(TimeSignature {
        numerator: num.trim().parse().ok()?,
        denominator: den.trim().parse().ok()?,
        symbol: None,
    })
}

fn parse_key(s: &str) -> Option<Key> {
    // Strip anything after whitespace (clef, transpose, etc. — not yet handled)
    let s = s.split_whitespace().next().unwrap_or(s);
    let mut chars = s.chars();
    let pitch = match chars.next()? {
        'C' => Pitch::C, 'D' => Pitch::D, 'E' => Pitch::E, 'F' => Pitch::F,
        'G' => Pitch::G, 'A' => Pitch::A, 'B' => Pitch::B,
        _ => return None,
    };
    let mode = match chars.as_str().to_lowercase().as_str() {
        "" | "maj" | "major" => Mode::Major,
        "m" | "min" | "minor" => Mode::Minor,
        "ion" | "ionian"     => Mode::Ionian,
        "aeo" | "aeolian"    => Mode::Aeolian,
        "dor" | "dorian"     => Mode::Dorian,
        "phr" | "phrygian"   => Mode::Phrygian,
        "lyd" | "lydian"     => Mode::Lydian,
        "mix" | "mixolydian" => Mode::Mixolydian,
        "loc" | "locrian"    => Mode::Locrian,
        _ => return None,
    };
    Some(Key { pitch, mode })
}

fn parse_tempo(s: &str) -> Option<Tempo> {
    let s = s.trim();
    // Strip optional trailing text like `"Andante"` or `"Allegro"`
    let s = s.find('"').map(|i| s[..i].trim()).unwrap_or(s);
    if s.is_empty() { return None; }

    if let Some(eq_pos) = s.find('=') {
        let beat_str = s[..eq_pos].trim();
        let bpm_str  = s[eq_pos + 1..].trim();
        let bpm: u16 = bpm_str.split_whitespace().next()?.parse().ok()?;
        let beat = parse_fraction(beat_str)?;
        Some(Tempo { bpm, beat_unit: Some(beat) })
    } else {
        // Plain BPM: `Q:120`
        let bpm: u16 = s.split_whitespace().next()?.parse().ok()?;
        Some(Tempo { bpm, beat_unit: None })
    }
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
    fn parses_phrygian_mode() {
        let tune = parse("M:4/4\nL:1/8\nK:Ephr").unwrap();
        assert!(matches!(tune.header.key.mode, Mode::Phrygian));
    }

    #[test]
    fn parses_lydian_mode() {
        let tune = parse("M:4/4\nL:1/8\nK:Flyd").unwrap();
        assert!(matches!(tune.header.key.mode, Mode::Lydian));
    }

    #[test]
    fn parses_locrian_mode() {
        let tune = parse("M:4/4\nL:1/8\nK:Bloc").unwrap();
        assert!(matches!(tune.header.key.mode, Mode::Locrian));
    }

    #[test]
    fn parses_aeolian_as_minor() {
        let tune = parse("M:4/4\nL:1/8\nK:Aaeo").unwrap();
        assert!(matches!(tune.header.key.mode, Mode::Aeolian));
    }

    #[test]
    fn parses_ionian_as_major() {
        let tune = parse("M:4/4\nL:1/8\nK:Cion").unwrap();
        assert!(matches!(tune.header.key.mode, Mode::Ionian));
    }

    #[test]
    fn parses_tempo_plain_bpm() {
        let tune = parse("M:4/4\nL:1/8\nK:C\nQ:120").unwrap();
        let tempo = tune.header.tempo.unwrap();
        assert_eq!(tempo.bpm, 120);
        assert!(tempo.beat_unit.is_none());
    }

    #[test]
    fn parses_tempo_with_beat_unit() {
        let tune = parse("M:4/4\nL:1/8\nK:C\nQ:1/4=100").unwrap();
        let tempo = tune.header.tempo.unwrap();
        assert_eq!(tempo.bpm, 100);
        let bu = tempo.beat_unit.unwrap();
        assert_eq!(bu.numerator, 1);
        assert_eq!(bu.denominator, 4);
    }

    #[test]
    fn parses_tempo_with_quoted_text() {
        let tune = parse("M:4/4\nL:1/8\nK:C\nQ:1/4=96 \"Andante\"").unwrap();
        assert_eq!(tune.header.tempo.unwrap().bpm, 96);
    }

    #[test]
    fn missing_tempo_is_none() {
        let tune = parse("M:4/4\nL:1/8\nK:C").unwrap();
        assert!(tune.header.tempo.is_none());
    }

    #[test]
    fn parses_rest_in_body() {
        let tune = parse("M:4/4\nL:1/8\nK:C\ncze").unwrap();
        let bars = plain_bars(&tune);
        assert_eq!(bars[0].elements.len(), 3);
        assert!(matches!(&bars[0].elements[0], BarElement::Note(_)));
        assert!(matches!(&bars[0].elements[1], BarElement::Rest(r) if !r.invisible));
        assert!(matches!(&bars[0].elements[2], BarElement::Note(_)));
    }

    #[test]
    fn parses_invisible_rest() {
        let tune = parse("M:4/4\nL:1/8\nK:C\nx2").unwrap();
        let bars = plain_bars(&tune);
        assert!(matches!(&bars[0].elements[0], BarElement::Rest(r) if r.invisible && r.duration.numerator == 2));
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
    fn broken_rhythm_right() {
        // A>d: A gets 3/2, d gets 1/2
        let tune = parse("M:4/4\nL:1/8\nK:C\nA>d").unwrap();
        let bars = plain_bars(&tune);
        let BarElement::Note(long) = &bars[0].elements[0] else { panic!() };
        let BarElement::Note(short) = &bars[0].elements[1] else { panic!() };
        assert_eq!(long.duration.numerator,   3);
        assert_eq!(long.duration.denominator, 2);
        assert_eq!(short.duration.numerator,  1);
        assert_eq!(short.duration.denominator,2);
    }

    #[test]
    fn broken_rhythm_left() {
        // A<d: A gets 1/2, d gets 3/2
        let tune = parse("M:4/4\nL:1/8\nK:C\nA<d").unwrap();
        let bars = plain_bars(&tune);
        let BarElement::Note(short) = &bars[0].elements[0] else { panic!() };
        let BarElement::Note(long)  = &bars[0].elements[1] else { panic!() };
        assert_eq!(short.duration.denominator, 2);
        assert_eq!(long.duration.numerator,    3);
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
