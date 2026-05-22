#![allow(dead_code)]

#[derive(Debug, PartialEq, Clone)]
pub enum Accidental {
    Sharp,
    Flat,
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

// (p:q:r) — p notes in the time of q, affecting r notes. q and r are optional.
#[derive(Debug)]
pub struct Tuplet {
    pub p: u8,
    pub q: Option<u8>,
    pub r: Option<u8>,
}

#[derive(Debug)]
pub enum Token {
    Note(Note),
    Header(char, String),
    Tuplet(Tuplet),
    Grace(Vec<Note>, bool), // notes, acciaccatura
    Volta(u8),
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
}

#[derive(Debug)]
pub enum BarElement {
    Note(Note),
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
    Mixolydian,
}

#[derive(Debug)]
pub struct Key {
    pub pitch: Pitch,
    pub mode: Mode,
}

#[derive(Debug)]
pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
}