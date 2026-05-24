# ABC 2.1 Standard Coverage

Reference: [ABC Notation Standard v2.1](https://abcnotation.com/wiki/abc:standard:v2.1/)

Legend: ✅ Implemented · ⚠️ Partial · ❌ Not implemented

---

## 1. Information Fields (Headers)

| Field | Name | Status | Notes |
|-------|------|--------|-------|
| `X:` | Reference Number | ⚠️ | Lexed but not stored or emitted |
| `T:` | Tune Title | ✅ | Stored and emitted in `\header { title = ... }` |
| `M:` | Meter | ✅ | Including `C` (common) and `C\|` (cut) symbols |
| `L:` | Unit Note Length | ✅ | Used as the duration baseline throughout |
| `K:` | Key Signature | ⚠️ | See §2 below |
| `Q:` | Tempo | ✅ | Plain BPM (`Q:120`), note-unit form (`Q:1/4=120`), dotted (`Q:3/8=80`), trailing text stripped |
| `C:` | Composer | ❌ | Silently ignored |
| `O:` | Origin | ❌ | Silently ignored |
| `A:` | Area (deprecated) | ❌ | Silently ignored |
| `R:` | Rhythm | ❌ | Silently ignored |
| `P:` | Parts | ❌ | Silently ignored; see §8 |
| `V:` | Voice | ❌ | Silently ignored; see §9 |
| `w:` | Words (aligned) | ❌ | Silently ignored; see §10 |
| `W:` | Words (after tune) | ❌ | Silently ignored; see §10 |
| `N:` | Notes | ❌ | Silently ignored |
| `H:` | History | ❌ | Silently ignored |
| `Z:` | Transcription | ❌ | Silently ignored |
| `B:` | Book | ❌ | Silently ignored |
| `D:` | Discography | ❌ | Silently ignored |
| `F:` | File URL | ❌ | Silently ignored |
| `S:` | Source | ❌ | Silently ignored |
| `G:` | Group | ❌ | Silently ignored |
| `I:` | Instruction/Directive | ❌ | Silently ignored; see §11 |
| `m:` | Macro | ❌ | Silently ignored; see §12 |
| `r:` | Remark | ❌ | Silently ignored |
| `s:` | Symbol Line | ❌ | Silently ignored |
| `U:` | User Defined Symbol | ❌ | Silently ignored |

### Inline fields
| Feature | Status | Notes |
|---------|--------|-------|
| `[M:...]` mid-tune meter change | ❌ | `[` in music currently returns `Token::Unknown` or Volta |
| `[L:...]` mid-tune length change | ❌ | |
| `[K:...]` mid-tune key change | ❌ | |
| `[Q:...]` mid-tune tempo change | ❌ | |
| `[V:...]` inline voice switch | ❌ | |
| Field continuation `+:` | ❌ | Not handled |

---

## 2. Key Signatures (`K:`)

### Modes
| Mode | Status | Notes |
|------|--------|-------|
| Major (`maj`, `major`, empty) | ✅ | |
| Minor (`m`, `min`, `minor`) | ✅ | |
| Dorian (`dor`, `dorian`) | ✅ | |
| Mixolydian (`mix`, `mixolydian`) | ✅ | |
| Phrygian (`phr`, `phrygian`) | ✅ | Correct key-sig offsets |
| Lydian (`lyd`, `lydian`) | ✅ | Correct key-sig offsets |
| Locrian (`loc`, `locrian`) | ✅ | Correct key-sig offsets |
| Aeolian (`aeo`, `aeolian`) | ✅ | Emits `\minor` (LilyPond has no separate `\aeolian`) |
| Ionian (`ion`, `ionian`) | ✅ | Emits `\major` (LilyPond has no separate `\ionian`) |

### Key modifiers & special keys
| Feature | Status | Notes |
|---------|--------|-------|
| Explicit accidentals in key: `K:D ^f` | ❌ | |
| `K:none` (no key signature) | ❌ | |
| `K:Hp` / `K:HP` (Highland pipes) | ❌ | |
| `K:exp` (explicit accidentals only) | ❌ | |
| Clef in key line: `K:G clef=bass` | ❌ | See §9 |

---

## 3. Notes

### Pitch & Octave
| Feature | Status | Notes |
|---------|--------|-------|
| Uppercase `C`–`B` (octave below middle) | ✅ | |
| Lowercase `c`–`b` (octave at/above middle) | ✅ | |
| Octave up with `'` | ✅ | Multiple supported |
| Octave down with `,` | ✅ | Multiple supported |

### Accidentals
| Feature | Status | Notes |
|---------|--------|-------|
| Sharp `^` | ✅ | |
| Flat `_` | ✅ | |
| Natural `=` | ✅ | Overrides key signature correctly |
| Double sharp `^^` | ✅ | Emits `isis` in LilyPond |
| Double flat `__` | ✅ | Emits `eses` in LilyPond |

### Durations
| Feature | Status | Notes |
|---------|--------|-------|
| Integer multiplier (`c2`, `c3`, …) | ✅ | |
| Bare slash `/` (halves) | ✅ | |
| Explicit `/N` | ✅ | |
| Multiple slashes `//` | ✅ | |
| Fraction `N/M` | ✅ | |
| Broken rhythm `>` / `<` | ✅ | Multi-arrow chains supported |

### Rests
| Feature | Status | Notes |
|---------|--------|-------|
| `z` (visible rest) | ✅ | Emits `r` with duration |
| `x` (invisible/spacer rest) | ✅ | Emits `s` with duration |
| `Z` (full-measure rest) | ⚠️ | Treated as `z` with duration; does not emit LilyPond `R1*N` multi-measure form |
| `X` (invisible full-measure rest) | ⚠️ | Same — treated as `x` with duration |

---

## 4. Barlines & Structure

| Feature | Status | Notes |
|---------|--------|-------|
| `\|` simple barline | ✅ | |
| `\|\|` double barline | ✅ | |
| `\|]` final barline | ✅ | Emits `\bar "\|."` |
| `\|:` repeat start | ✅ | |
| `:\|` repeat end | ✅ | |
| `::\|` / `:\|:` end+start | ✅ | |
| `[1`, `[2` volta brackets | ✅ | |
| `[1,3`, `[1-3` multi-volta | ❌ | Multi-number ranges not parsed |
| `[` thick-thin barline (`[\|`) | ✅ | Emitted as `\|` double bar |
| Dotted barlines | ❌ | |
| Invisible barlines | ❌ | |

---

## 5. Grace Notes & Ornaments

### Grace Notes
| Feature | Status | Notes |
|---------|--------|-------|
| Appoggiatura `{notes}` | ✅ | Emits `\appoggiatura` |
| Acciaccatura `{/notes}` | ✅ | Emits `\acciaccatura` |
| Multi-note grace `{gab}` | ✅ | Wrapped in `{ }` in LilyPond |

### Ornaments / Decorations
| Feature | Status | Notes |
|---------|--------|-------|
| `~` roll/turn | ✅ | Emits `\turn` |
| `!trill!` | ❌ | `!` not lexed |
| `!trill(!` / `!trill)!` | ❌ | |
| `!mordent!` / `!lowermordent!` / `!uppermordent!` | ❌ | |
| `!roll!` | ❌ | |
| `!fermata!` | ❌ | |
| `!segno!` / `!coda!` / `!fine!` | ❌ | |
| `!>!` accent / `!emphasis!` | ❌ | |
| Dynamic marks `!f!` `!ff!` `!mp!` `!pp!` etc. | ❌ | |
| `!crescendo(!` / `!crescendo)!` | ❌ | |
| `!diminuendo(!` / `!diminuendo)!` | ❌ | |
| `!D.C.!` / `!D.S.!` | ❌ | |
| Shorthand decorations: `H` `L` `M` `O` `P` `R` `S` `T` `u` `v` | ❌ | `!` syntax not lexed at all |
| User-defined via `U:` | ❌ | |

---

## 6. Tuplets

| Feature | Status | Notes |
|---------|--------|-------|
| Simple `(3` triplet | ✅ | |
| `(2` duplet | ✅ | |
| `(p:q` form | ✅ | |
| `(p:q:r` full form | ✅ | |
| Default `q` values per spec | ✅ | `p ∈ {2,4,8}` → `q=3`, else `q=2` |

---

## 7. Chords

| Feature | Status | Notes |
|---------|--------|-------|
| Guitar/chord symbols `"Am"` | ❌ | Quoted strings not lexed |
| Note clusters `[CEG]` | ❌ | `[` in body only handles Volta or `[\|` |

---

## 8. Parts (`P:`)

| Feature | Status | Notes |
|---------|--------|-------|
| Part labels `P:A`, `P:B`, etc. | ❌ | Ignored |
| Part repetition `P:AABB` | ❌ | Ignored |
| `P:` in tune body | ❌ | Ignored |

---

## 9. Voices & Clefs (`V:`)

| Feature | Status | Notes |
|---------|--------|-------|
| Multiple voices `V:1`, `V:2`, etc. | ❌ | No multi-staff output |
| Clef specification `clef=bass`, `clef=treble` | ❌ | |
| Clef variants: alto, tenor, soprano, perc, none | ❌ | |
| `transpose=N` | ❌ | |
| `octave=N` | ❌ | |
| `stafflines=N` | ❌ | |
| `middle=<pitch>` | ❌ | |

---

## 10. Lyrics

| Feature | Status | Notes |
|---------|--------|-------|
| `w:` aligned lyrics | ❌ | |
| `W:` words after tune | ❌ | |
| Lyric continuation `-` | ❌ | |
| Multiple syllables `*`, `_` | ❌ | |
| Hyphenation | ❌ | |

---

## 11. Typesetting Directives

| Feature | Status | Notes |
|---------|--------|-------|
| `%%` stylesheet directives | ❌ | Lines beginning `%%` fall through to `skip_line` via `%` comment handling |
| `I:` instruction fields | ❌ | Ignored (header lexed but not acted on) |

---

## 12. Macros & User Symbols

| Feature | Status | Notes |
|---------|--------|-------|
| `m:` macro definitions | ❌ | |
| Macro expansion in music body | ❌ | |
| `U:` user-defined symbol table | ❌ | |

---

## 13. File & Tune Structure

| Feature | Status | Notes |
|---------|--------|-------|
| Single tune | ✅ | |
| Multiple tunes per file | ❌ | Parser stops after first tune |
| File header (before first `X:`) | ❌ | Not handled |
| `%abc` version marker | ❌ | Treated as a comment (harmless) |
| Line continuation `\` at end of line | ❌ | Backslash not handled |
| Back-quote `` ` `` beam-break hint | ❌ | Falls through to `Token::Unknown` |

---

## 14. Emitter / LilyPond Output

| Feature | Status | Notes |
|---------|--------|-------|
| `\version "2.24.0"` | ✅ | |
| `\header { title = ... }` | ✅ | |
| Composer, arranger, etc. in `\header` | ❌ | |
| `\key` | ✅ | |
| `\time` | ✅ | |
| Tempo `\tempo` | ✅ | Emitted from `Q:` |
| `\partial` pickup bar | ✅ | Computed from first bar duration |
| `\repeat volta N { ... \alternative { ... } }` | ✅ | |
| `\bar "||"` double barline | ✅ | |
| `\bar "|."` final barline | ✅ | |
| Note accidentals | ✅ | |
| Key-signature-aware implicit accidentals | ✅ | |
| Accidental state tracking within a bar (sticky) | ❌ | Each note re-evaluated against key only |
| Grace notes / acciaccatura | ✅ | |
| Tuplets | ✅ | |
| `\turn` ornament | ✅ | |
| All other ornaments/dynamics | ❌ | |
| Style file injection (`style.ly`) | ✅ | |
| Multi-staff `\new Staff` / `<<>>` | ❌ | |
| Lyrics `\addlyrics` | ❌ | |

---

## Rough Priority Order for Next Work

Previously high-priority items now done: rests (`z`/`x`/`Z`/`X`), all 9 key modes, tempo (`Q:`), double accidentals (`^^`/`__`).

1. **Decorations / `!` syntax** — trill, fermata, dynamics are very common; `!` not lexed at all
2. **Chord symbols `"..."`** — ubiquitous in folk/session tunes; quoted strings not lexed
3. **Note clusters `[CEG]`** — needed for keyboard/harp transcription
4. **Inline field changes** (`[M:...]`, `[K:...]`, etc.) — needed for medleys and mid-tune key changes
5. **Accidental state tracking within a bar** — ABC accidentals are sticky for the bar; currently each note is re-evaluated against the key sig only
6. **Lyrics** (`w:`, `W:`)
7. **Multi-voice / multi-staff** (`V:`)
8. **Parts** (`P:`)
9. **Multiple tunes per file**
10. **`Z`/`X` as true LilyPond multi-measure rests** (`R1*N`) — requires knowing time signature
11. **Directives** (`%%`, `I:`)
12. **Macros** (`m:`, `U:`)
