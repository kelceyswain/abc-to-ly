# abc-to-ly

Converts [ABC notation](https://abcnotation.com/) music files to [LilyPond](https://lilypond.org/) `.ly` files, with optional compilation to PDF or SVG.

## Installation

```sh
cargo install --path .
```

This installs the binary as `abc-to-ly`. LilyPond must be installed separately if you want to compile output.

## Usage

```
abc-to-ly [OPTIONS] <INPUT>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<INPUT>` | Input `.abc` file |

### Options

| Option | Description |
|--------|-------------|
| `-o`, `--output <FILE>` | Output `.ly` file (default: same name as input) |
| `-s`, `--style <FILE>` | LilyPond style file inserted after `\version` |
| `-c`, `--compile` | Compile to PDF via `lilypond` |
| `--svg` | Compile to SVG via `lilypond` (implies `--compile`) |

### Examples

```sh
# Convert to .ly
abc-to-ly tune.abc

# Specify output path
abc-to-ly tune.abc -o output/tune.ly

# Convert and compile to PDF
abc-to-ly tune.abc -c

# Convert and compile to SVG with a style file
abc-to-ly tune.abc --svg -s style.ly
```

## Style files

A style file is any valid LilyPond snippet, inserted verbatim after the `\version` line and before `\header`. Use it to set paper size, staff size, fonts, and so on. For example:

> **Security note:** The style file is copied into the generated `.ly` output without any validation or sanitisation, and is executed by LilyPond as code. Only use style files from sources you trust.

```lilypond
#(set-global-staff-size 18)
\paper {
  line-width = 140\mm
  indent = 0\mm
  tagline = ##f
}
```

## Supported ABC features

| Feature | Example |
|---------|---------|
| Headers: T, M, L, K, R, X, Z, S | `T:The Morning Dew` |
| Time signatures | `M:6/8`, `M:C`, `M:C\|` |
| Key signatures and modes | `K:Gmaj`, `K:Am`, `K:Ddor`, `K:Gmix` |
| Notes with octave modifiers | `C D e f g' A,` |
| Accidentals | `^f _b =c` |
| Note durations | `A2 B/ c3/2` |
| Barlines | `\|`, `\|\|`, `\|]` |
| Repeats | `\|: ... :\|` |
| First/second endings | `[1 ... [2 ...` or `\|1 ... \|2 ...` |
| Ornaments | `~G` (turn) |
| Grace notes | `{g}a` (appoggiatura), `{/g}a` (acciaccatura) |
| Broken rhythm | `A>d`, `A<<d` |
| Tuplets | `(3abc`, `(3:2:3abc` |
| Pickup bars | detected automatically, emits `\partial` |
| Key signature applied implicitly | `f` in D major → `fis` in LilyPond |

## Notes

- LilyPond uses absolute pitch notation; octave mapping: ABC uppercase `C` = middle C = LilyPond `c'`.
- `M:C` emits `\time 4/4`; LilyPond renders the common-time C symbol automatically.
- `lilypond` is invoked from `PATH`; if not found, the `.ly` file is still written and a warning is printed.
