#[derive(Debug, PartialEq, Clone)]
pub enum Accidental {
    Sharp,
    DoubleSharp,
    Flat,
    DoubleFlat,
    Natural,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Pitch {
    A, B, C, D, E, F, G,
}

#[derive(Debug)]
pub struct Duration {
    pub numerator: u8,
    pub denominator: u8,
}

#[derive(Debug)]
pub enum Ornament {
    Turn,
}

#[derive(Debug)]
pub struct Grace {
    pub notes: Vec<Note>,
    pub acciaccatura: bool,
}

#[derive(Debug)]
pub struct Note {
    pub pitch: Pitch,
    pub octave: i8,  // 0 = middle octave, +1/-1 etc
    pub accidental: Option<Accidental>,
    pub ornament: Option<Ornament>,
    pub grace: Option<Grace>,
    pub duration: Duration, // relative to L: default, so 1 = default, 2 = double
}

#[derive(Debug)]
pub struct Rest {
    pub duration: Duration,
    pub invisible: bool, // z = false, x = true
}

// (p:q:r) — p notes in the time of q, affecting r notes. q and r are optional.
#[derive(Debug)]
pub struct Tuplet {
    pub p: u8,
    pub q: Option<u8>,
    pub r: Option<u8>,
}

#[derive(Debug)]
pub struct Tempo {
    pub bpm: u16,
    pub beat_unit: Option<Duration>, // None = use L: (default note length)
}

#[derive(Debug)]
pub enum Token {
    Note(Note),
    Rest(Rest),
    Header(char, String),
    Tuplet(Tuplet),
    Grace(Vec<Note>, bool), // notes, acciaccatura
    BrokenRight,            // >
    BrokenLeft,             // <
    // The number is lexed but the parser currently orders alternatives by position
    // rather than by volta label — see parser::parse_sections.
    Volta(#[allow(dead_code)] u8),
    RepeatStart,
    RepeatEnd,
    RepeatEndStart,
    Bar,
    DoubleBar,
    FinalBar,
    Unknown,
}

#[derive(Debug)]
pub enum Section {
    Plain(Vec<Bar>),
    Repeat { body: Vec<Bar>, alternatives: Vec<Vec<Bar>> },
    DoubleBar,
}

#[derive(Debug)]
pub struct Tune {
    pub header: Header,
    pub sections: Vec<Section>,
    pub final_bar: bool,
}

#[derive(Debug)]
pub struct Header {
    pub title: String,
    pub key: Key,
    pub time: TimeSignature,
    pub default_length: Duration,
    pub tempo: Option<Tempo>,
}

#[derive(Debug)]
pub enum BarElement {
    Note(Note),
    Rest(Rest),
    Tuplet(Tuplet, Vec<Note>),
}

#[derive(Debug)]
pub struct Bar {
    pub elements: Vec<BarElement>,
}

#[derive(Debug)]
pub enum Mode {
    Major,
    Minor,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Locrian,
    Aeolian,
    Ionian,
}

#[derive(Debug)]
pub struct Key {
    pub pitch: Pitch,
    pub mode: Mode,
}

/// Parsed from `M:C` / `M:C|` but the emitter currently always emits `\time n/d`
/// numerically.  Reserved for when the emitter grows `\commonTime` / `\cutTime` support.
#[allow(dead_code)]
#[derive(Debug)]
pub enum TimeSymbol { Common, Cut }

#[derive(Debug)]
pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
    #[allow(dead_code)]   // see TimeSymbol above
    pub symbol: Option<TimeSymbol>,
}