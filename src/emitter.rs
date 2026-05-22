use crate::ast::{Accidental, Bar, BarElement, Duration, Grace, Key, Mode, Note, Ornament, Pitch, Section, TimeSignature, Tune, Tuplet};

pub fn emit(tune: &Tune, style: Option<&str>) -> String {
    let mut out = String::new();

    out.push_str("\\version \"2.24.0\"\n\n");

    if let Some(s) = style {
        out.push_str(s.trim_end());
        out.push_str("\n\n");
    }

    if !tune.header.title.is_empty() {
        out.push_str(&format!(
            "\\header {{\n  title = \"{}\"\n}}\n\n",
            tune.header.title
        ));
    }

    out.push_str("\\score {\n  \\new Staff {\n");
    out.push_str(&format!("    {}\n", emit_key(&tune.header.key)));
    out.push_str(&format!("    {}\n", emit_time(&tune.header.time)));

    let key_sig = KeySig::from_key(&tune.header.key);
    let dl = &tune.header.default_length;
    let time = &tune.header.time;

    for section in &tune.sections {
        match section {
            Section::DoubleBar => {
                out.push_str("    \\bar \"||\"\n");
                continue;
            }
            Section::Plain(bars) => {
                if let Some(dur) = pickup_duration(bars, time, dl) {
                    out.push_str(&format!("    \\partial {}\n", dur));
                }
                for bar in bars {
                    out.push_str(&format!("    {} |\n", emit_bar(bar, dl, &key_sig)));
                }
            }
            Section::Repeat { body, alternatives } => {
                let n_voltas = alternatives.len().max(2);
                out.push_str(&format!("    \\repeat volta {} {{\n", n_voltas));
                if let Some(dur) = pickup_duration(body, time, dl) {
                    out.push_str(&format!("      \\partial {}\n", dur));
                }
                for bar in body {
                    out.push_str(&format!("      {} |\n", emit_bar(bar, dl, &key_sig)));
                }
                if !alternatives.is_empty() {
                    out.push_str("      \\alternative {\n");
                    for alt in alternatives {
                        let bars_str: Vec<String> = alt.iter()
                            .map(|b| format!("{} |", emit_bar(b, dl, &key_sig)))
                            .collect();
                        out.push_str(&format!("        {{ {} }}\n", bars_str.join(" ")));
                    }
                    out.push_str("      }\n");
                }
                out.push_str("    }\n");
            }
        }
    }

    if tune.final_bar {
        out.push_str("    \\bar \"|.\"\n");
    }
    out.push_str("  }\n}\n");
    out
}

// Which pitches are implicitly sharp or flat in a given key.
struct KeySig {
    sharps: Vec<Pitch>,
    flats: Vec<Pitch>,
}

impl KeySig {
    fn from_key(key: &Key) -> Self {
        let tonic = pitch_semitone(&key.pitch) as i32;
        // Offset to find the equivalent Ionian (major) tonic
        let offset: i32 = match key.mode {
            Mode::Major => 0,
            Mode::Minor => 3,
            Mode::Dorian => 10,
            Mode::Mixolydian => 5,
        };
        let major_tonic = (tonic + offset).rem_euclid(12) as u8;

        let sharp_order = [Pitch::F, Pitch::C, Pitch::G, Pitch::D, Pitch::A, Pitch::E, Pitch::B];
        let flat_order  = [Pitch::B, Pitch::E, Pitch::A, Pitch::D, Pitch::G, Pitch::C, Pitch::F];

        let (sharps, flats): (Vec<Pitch>, Vec<Pitch>) = match major_tonic {
            7  => (sharp_order[..1].to_vec(), vec![]),
            2  => (sharp_order[..2].to_vec(), vec![]),
            9  => (sharp_order[..3].to_vec(), vec![]),
            4  => (sharp_order[..4].to_vec(), vec![]),
            11 => (sharp_order[..5].to_vec(), vec![]),
            6  => (sharp_order[..6].to_vec(), vec![]),
            5  => (vec![], flat_order[..1].to_vec()),
            10 => (vec![], flat_order[..2].to_vec()),
            3  => (vec![], flat_order[..3].to_vec()),
            8  => (vec![], flat_order[..4].to_vec()),
            1  => (vec![], flat_order[..5].to_vec()),
            _  => (vec![], vec![]),  // C major / no accidentals
        };
        Self { sharps, flats }
    }

    // Returns the LilyPond accidental suffix to append after the pitch name.
    fn acc_suffix(&self, pitch: &Pitch) -> &str {
        if self.sharps.contains(pitch) { "is" }
        else if self.flats.contains(pitch) { "es" }
        else { "" }
    }
}

fn pitch_semitone(pitch: &Pitch) -> u8 {
    match pitch {
        Pitch::C => 0, Pitch::D => 2, Pitch::E => 4, Pitch::F => 5,
        Pitch::G => 7, Pitch::A => 9, Pitch::B => 11,
    }
}

fn emit_key(key: &Key) -> String {
    let mode = match key.mode {
        Mode::Major => "major",
        Mode::Minor => "minor",
        Mode::Dorian => "dorian",
        Mode::Mixolydian => "mixolydian",
    };
    format!("\\key {} \\{}", pitch_name(&key.pitch), mode)
}

fn emit_time(time: &TimeSignature) -> String {
    format!("\\time {}/{}", time.numerator, time.denominator)
}

fn emit_bar(bar: &Bar, default_len: &Duration, key_sig: &KeySig) -> String {
    bar.elements
        .iter()
        .map(|el| match el {
            BarElement::Note(n) => emit_note(n, default_len, key_sig),
            BarElement::Tuplet(t, notes) => emit_tuplet(t, notes, default_len, key_sig),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn emit_grace(grace: &Grace, key_sig: &KeySig) -> String {
    let keyword = if grace.acciaccatura { "\\acciaccatura" } else { "\\appoggiatura" };
    // Grace notes are always eighth notes in LilyPond
    let eighth_len = Duration { numerator: 1, denominator: 8 };
    let notes: Vec<String> = grace.notes.iter()
        .map(|n| emit_note(n, &eighth_len, key_sig))
        .collect();
    if notes.len() == 1 {
        format!("{} {} ", keyword, notes[0])
    } else {
        format!("{} {{ {} }} ", keyword, notes.join(" "))
    }
}

fn emit_note(note: &Note, default_len: &Duration, key_sig: &KeySig) -> String {
    let grace_prefix = note.grace.as_ref()
        .map(|g| emit_grace(g, key_sig))
        .unwrap_or_default();
    let acc_suffix = match &note.accidental {
        Some(Accidental::Sharp)   => "is",
        Some(Accidental::Flat)    => "es",
        Some(Accidental::Natural) => "",    // plain pitch name = natural; LilyPond prints the sign
        None => key_sig.acc_suffix(&note.pitch),
    };
    let ornament = match note.ornament {
        Some(Ornament::Turn) => "\\turn",
        None => "",
    };
    format!(
        "{}{}{}{}{}{}",
        grace_prefix,
        pitch_name(&note.pitch),
        acc_suffix,
        emit_octave(note.octave),
        lily_duration(&note.duration, default_len),
        ornament,
    )
}

fn emit_tuplet(tuplet: &Tuplet, notes: &[Note], default_len: &Duration, key_sig: &KeySig) -> String {
    let q = tuplet.q.unwrap_or(default_q(tuplet.p));
    let inner = notes
        .iter()
        .map(|n| emit_note(n, default_len, key_sig))
        .collect::<Vec<_>>()
        .join(" ");
    format!("\\tuplet {}/{} {{ {} }}", tuplet.p, q, inner)
}

fn pitch_name(pitch: &Pitch) -> &'static str {
    match pitch {
        Pitch::C => "c", Pitch::D => "d", Pitch::E => "e", Pitch::F => "f",
        Pitch::G => "g", Pitch::A => "a", Pitch::B => "b",
    }
}

fn emit_octave(octave: i8) -> String {
    // ABC uppercase C = C4 (middle C) = LilyPond c' (one apostrophe).
    // ABC octave -1 → lily 1, ABC octave 0 → lily 2, etc.
    let n = octave + 2;
    if n > 0 {
        "'".repeat(n as usize)
    } else if n < 0 {
        ",".repeat((-n) as usize)
    } else {
        String::new()
    }
}

fn lily_duration(note: &Duration, default_len: &Duration) -> String {
    let num = (note.numerator as u32) * (default_len.numerator as u32);
    let den = (note.denominator as u32) * (default_len.denominator as u32);
    let g = gcd(num, den);
    match (num / g, den / g) {
        (1, d) => format!("{}", d),
        (3, d) => format!("{}.", d / 2),
        (n, d) => format!("{}", d / n),
    }
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn default_q(p: u8) -> u8 {
    match p { 2 | 4 | 8 => 3, _ => 2 }
}

// Total duration of a bar as a reduced fraction in L: units.
fn bar_l_units(bar: &Bar) -> (u32, u32) {
    let (mut n, mut d) = (0u32, 1u32);
    for elem in &bar.elements {
        let (en, ed) = match elem {
            BarElement::Note(note) => {
                (note.duration.numerator as u32, note.duration.denominator as u32)
            }
            BarElement::Tuplet(t, notes) => {
                let q = t.q.unwrap_or(default_q(t.p)) as u32;
                let p = t.p as u32;
                let (mut tn, mut td) = (0u32, 1u32);
                for note in notes {
                    let nn = note.duration.numerator as u32 * q;
                    let nd = note.duration.denominator as u32 * p;
                    tn = tn * nd + nn * td;
                    td *= nd;
                    let g = gcd(tn, td);
                    tn /= g; td /= g;
                }
                (tn, td)
            }
        };
        n = n * ed + en * d;
        d *= ed;
        let g = gcd(n, d);
        n /= g; d /= g;
    }
    (n, d)
}

// Returns a LilyPond \partial duration string if the first bar is shorter than a full bar.
fn pickup_duration(bars: &[Bar], time: &TimeSignature, dl: &Duration) -> Option<String> {
    let first = bars.first()?;
    let (bar_n, bar_d) = bar_l_units(first);
    if bar_n == 0 { return None; }

    // Full bar in L: units = (M_num * L_den) / (M_den * L_num)
    let full_n = time.numerator as u32 * dl.denominator as u32;
    let full_d = time.denominator as u32 * dl.numerator as u32;

    // Pickup if bar_n/bar_d < full_n/full_d
    if bar_n * full_d >= full_n * bar_d { return None; }

    // Pickup duration in whole notes = bar_n/bar_d * L_num/L_den
    let pn = bar_n * dl.numerator as u32;
    let pd = bar_d * dl.denominator as u32;
    let g = gcd(pn, pd);
    let (pn, pd) = (pn / g, pd / g);

    let dur = match (pn, pd) {
        (1, d) => format!("{}", d),
        (3, d) => format!("{}.", d / 2),
        _ => return None,
    };
    Some(dur)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn emit_str(input: &str) -> String {
        let tune = Parser::new(Lexer::new(input)).parse().unwrap();
        emit(&tune, None)
    }

    #[test]
    fn emits_version() {
        assert!(emit_str("M:4/4\nL:1/8\nK:D").contains("\\version \"2.24.0\""));
    }

    #[test]
    fn emits_title() {
        let out = emit_str("T:The Morning Dew\nM:4/4\nL:1/8\nK:D");
        assert!(out.contains("title = \"The Morning Dew\""));
    }

    #[test]
    fn no_header_block_when_no_title() {
        let out = emit_str("M:4/4\nL:1/8\nK:D");
        assert!(!out.contains("\\header"));
    }

    #[test]
    fn emits_key_major() {
        assert!(emit_str("M:4/4\nL:1/8\nK:D").contains("\\key d \\major"));
    }

    #[test]
    fn emits_key_minor() {
        assert!(emit_str("M:4/4\nL:1/8\nK:Am").contains("\\key a \\minor"));
    }

    #[test]
    fn emits_key_dorian() {
        assert!(emit_str("M:4/4\nL:1/8\nK:Ddor").contains("\\key d \\dorian"));
    }

    #[test]
    fn emits_time_sig() {
        assert!(emit_str("M:6/8\nL:1/8\nK:G").contains("\\time 6/8"));
    }

    // --- octave mapping: uppercase C = C4 = lily c', lowercase c = C5 = lily c'' ---

    #[test]
    fn emits_uppercase_as_middle_octave() {
        assert!(emit_str("M:4/4\nL:1/4\nK:C\nC").contains("c'4"));
    }

    #[test]
    fn emits_lowercase_one_above_middle() {
        assert!(emit_str("M:4/4\nL:1/4\nK:C\nc").contains("c''4"));
    }

    #[test]
    fn emits_abc_octave_up_modifier() {
        // c' in ABC = C6 = lily c'''
        assert!(emit_str("M:4/4\nL:1/4\nK:C\nc'").contains("c'''4"));
    }

    #[test]
    fn emits_abc_octave_down_modifier() {
        // C, in ABC = C3 = lily c
        assert!(emit_str("M:4/4\nL:1/4\nK:C\nC,").contains("c4"));
    }

    // --- accidentals ---

    #[test]
    fn emits_explicit_sharp() {
        assert!(emit_str("M:4/4\nL:1/8\nK:C\n^c").contains("cis''8"));
    }

    #[test]
    fn emits_explicit_flat() {
        assert!(emit_str("M:4/4\nL:1/8\nK:C\n_b").contains("bes''8"));
    }

    #[test]
    fn emits_natural_overrides_key_sig() {
        // =f in D major (F# key sig) should emit plain f, not fis
        let out = emit_str("M:4/4\nL:1/8\nK:D\n=f");
        assert!(out.contains("f''8"));
        assert!(!out.contains("fis"));
    }

    // --- key signature applied implicitly ---

    #[test]
    fn key_sig_sharpens_f_in_d_major() {
        // f with no accidental in D major should emit fis
        assert!(emit_str("M:4/4\nL:1/8\nK:D\nf").contains("fis''8"));
    }

    #[test]
    fn key_sig_sharpens_c_in_d_major() {
        assert!(emit_str("M:4/4\nL:1/8\nK:D\nc").contains("cis''8"));
    }

    #[test]
    fn key_sig_flattens_b_in_f_major() {
        assert!(emit_str("M:4/4\nL:1/8\nK:F\nb").contains("bes''8"));
    }

    #[test]
    fn c_major_has_no_key_sig_accidentals() {
        let out = emit_str("M:4/4\nL:1/8\nK:C\nc d e f g a b");
        assert!(!out.contains("is") && !out.contains("es"));
    }

    // --- durations ---

    #[test]
    fn emits_duration_default() {
        assert!(emit_str("M:4/4\nL:1/8\nK:C\nc").contains("c''8"));
    }

    #[test]
    fn emits_duration_double() {
        assert!(emit_str("M:4/4\nL:1/8\nK:C\nc2").contains("c''4"));
    }

    #[test]
    fn emits_duration_half() {
        assert!(emit_str("M:4/4\nL:1/8\nK:C\nc/2").contains("c''16"));
    }

    #[test]
    fn emits_duration_dotted() {
        assert!(emit_str("M:4/4\nL:1/8\nK:C\nc3/2").contains("c''8."));
    }

    #[test]
    fn emits_bar_separators() {
        let out = emit_str("M:4/4\nL:1/4\nK:C\nc | d");
        assert_eq!(out.matches(" |").count(), 2);
    }

    #[test]
    fn emits_appoggiatura() {
        let out = emit_str("M:6/8\nL:1/8\nK:G\n{g}a");
        assert!(out.contains("\\appoggiatura g''8 a''8"));
    }

    #[test]
    fn emits_acciaccatura() {
        let out = emit_str("M:6/8\nL:1/8\nK:G\n{/g}a");
        assert!(out.contains("\\acciaccatura g''8 a''8"));
    }

    #[test]
    fn emits_multi_note_grace() {
        let out = emit_str("M:6/8\nL:1/8\nK:G\n{ga}b");
        assert!(out.contains("\\appoggiatura { g''8 a''8 } b''8"));
    }

    #[test]
    fn emits_turn_ornament() {
        assert!(emit_str("M:6/8\nL:1/8\nK:G\n~G").contains("g'8\\turn"));
    }

    #[test]
    fn emits_tuplet() {
        assert!(emit_str("M:4/4\nL:1/8\nK:C\n(3cde").contains("\\tuplet 3/2 { c''8 d''8 e''8 }"));
    }

    #[test]
    fn emits_partial_for_two_note_pickup() {
        // 2 eighth notes in 6/8 = \partial 4
        let out = emit_str("M:6/8\nL:1/8\nK:G\n|:cd|efgabc:|");
        assert!(out.contains("\\partial 4\n"));
    }

    #[test]
    fn no_partial_for_full_first_bar() {
        let out = emit_str("M:6/8\nL:1/8\nK:G\n|:abcdef|gabcde:|");
        assert!(!out.contains("\\partial"));
    }

    #[test]
    fn full_output() {
        let out = emit_str("T:The Morning Dew\nM:4/4\nL:1/8\nK:D\nabc | def");
        let expected = "\
\\version \"2.24.0\"

\\header {
  title = \"The Morning Dew\"
}

\\score {
  \\new Staff {
    \\key d \\major
    \\time 4/4
    \\partial 4.
    a''8 b''8 cis''8 |
    d''8 e''8 fis''8 |
  }
}
";
        assert_eq!(out, expected);
    }
}
