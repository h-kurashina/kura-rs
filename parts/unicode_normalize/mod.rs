//! Unicode cleanup for text pipelines: NFC / NFD / NFKC / NFKD normalization,
//! whitespace folding and removal of invisible characters.
//!
//! Normalization is bit-for-bit the same as Python's
//! `unicodedata.normalize(form, text)` on Python 3.14 (Unicode 16.0.0), and
//! passes the official `NormalizationTest.txt` for Unicode 16.0.0.
//!
//! ```ignore
//! use unicode_normalize::{normalize, Form, Options};
//!
//! // The whole pipeline: remove invisible characters, NFKC, fold whitespace
//! let clean = normalize("\u{FEFF}ﾃﾞｰﾀ\u{3000}\u{3000}ＡＩ\u{200B}  ", Options::nfkc().fold_whitespace().strip_invisible());
//! assert_eq!(clean, "データ AI");
//!
//! // Normalization only (same as unicodedata.normalize("NFC", text))
//! assert_eq!(normalize("e\u{301}", Options::nfc()), "\u{E9}");
//! assert!(unicode_normalize::is_normalized("\u{E9}", Form::Nfc));
//! ```
//!
//! # Unicode version
//!
//! Normalization data comes from the `unicode-normalization` crate. Its version
//! decides the Unicode version, so the dependency is pinned exactly:
//! `unicode-normalization = "=0.1.24"` implements Unicode 16.0.0, the same as
//! Python 3.14's `unicodedata.unidata_version` (Python 3.13 is 15.1.0, 3.12 is
//! 15.0.0). 0.1.25 moved to Unicode 17.0.0, which changes 35 code points: U+A7F1
//! gains a compatibility decomposition (NFKC gives "S") and 34 new combining
//! marks (in U+1ACF–U+1AEB, U+10EFA–U+10EFB and U+1E6E3–U+1E6F5) get a
//! nonzero combining class, so they are reordered next to other marks. With 0.1.25,
//! 283 of the verification cases differ from Python 3.14. Move to 0.1.25 only
//! together with a Python whose `unicodedata` is 17.0.0.
//! [`UNICODE_VERSION`] tells which version was compiled in.
//!
//! # What the pipeline does
//!
//! [`normalize`] applies the steps that are switched on in [`Options`], in this
//! order (the order matters, see below):
//!
//! 1. **strip invisible** ([`strip_invisible`]): delete every character in
//!    [`INVISIBLE`]. The set is exactly the characters that are both
//!    `General_Category = Cf` (format) and `Default_Ignorable_Code_Point` in
//!    Unicode 16.0.0: soft hyphen, Arabic letter mark, Mongolian vowel
//!    separator, zero-width space / non-joiner / joiner, LRM / RLM, bidi
//!    embeddings, overrides and isolates, word joiner, invisible math
//!    operators, deprecated format characters, BOM (U+FEFF), shorthand format
//!    controls, musical beam/tie/slur controls and tag characters (U+E0001,
//!    U+E0020–U+E007F, used to hide text from people in LLM prompts).
//!    *Not* removed: variation selectors, combining grapheme joiner (U+034F),
//!    Hangul fillers, Mongolian free variation selectors, visible format
//!    characters such as U+0600 ARABIC NUMBER SIGN, unassigned code points,
//!    and whitespace. Removing ZWJ splits emoji ZWJ sequences
//!    (👨‍👩‍👧 → 👨👩👧) and removing tags turns subdivision flags into 🏴;
//!    leave `strip_invisible` off if that matters.
//! 2. **normalize** to the chosen [`Form`] (none by default).
//! 3. **fold whitespace** ([`fold_whitespace`]): replace every run of
//!    whitespace with a single U+0020 SPACE and remove whitespace at both
//!    ends. Whitespace means exactly Python's `str.isspace()`
//!    ([`WHITESPACE`], 29 characters: tab, line feed, vertical tab, form feed,
//!    carriage return, U+001C–U+001F, space, U+0085, NBSP, U+1680,
//!    U+2000–U+200A, line and paragraph separators, U+202F, U+205F and the
//!    ideographic space U+3000). This is Rust's `char::is_whitespace` plus
//!    U+001C–U+001F. Python equivalent: `" ".join(text.split())`.
//!
//! Invisible characters are removed first so that `"a\u{200B} b"` folds to
//! `"a b"` and a letter and its accent that were separated by one are
//! normalized together. Whitespace is folded last because NFKC turns NBSP, U+2000–U+200A
//! and U+3000 into U+0020. With this order the output is always in the chosen
//! form, contains no invisible characters, and running the pipeline again
//! changes nothing.
//!
//! Every function that returns text returns `Cow::Borrowed` when the input is
//! already clean, so clean text is never copied.
//!
//! Dependency: `unicode-normalization = "=0.1.24"`.
//!
//! Part of kura-rs (https://github.com/h-kurashina/kura-rs). MIT OR Apache-2.0.

use std::borrow::Cow;
use std::fmt;
use std::iter;
use std::str::FromStr;
use std::sync::OnceLock;

use unicode_normalization::char::canonical_combining_class;
use unicode_normalization::{
    IsNormalized, UnicodeNormalization, is_nfc_quick, is_nfd_quick, is_nfkc_quick, is_nfkd_quick,
};

/// The Unicode version of the normalization data, e.g. `(16, 0, 0)`.
/// Compare it with Python's `unicodedata.unidata_version`.
pub const UNICODE_VERSION: (u8, u8, u8) = unicode_normalization::UNICODE_VERSION;

/// A Unicode normalization form (UAX #15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Form {
    /// Canonical decomposition, then canonical composition.
    Nfc,
    /// Canonical decomposition.
    Nfd,
    /// Compatibility decomposition, then canonical composition.
    Nfkc,
    /// Compatibility decomposition.
    Nfkd,
}

impl Form {
    /// Every form.
    pub const ALL: [Form; 4] = [Form::Nfc, Form::Nfd, Form::Nfkc, Form::Nfkd];

    /// `"NFC"`, `"NFD"`, `"NFKC"` or `"NFKD"`, the names `unicodedata` uses.
    pub fn name(self) -> &'static str {
        match self {
            Form::Nfc => "NFC",
            Form::Nfd => "NFD",
            Form::Nfkc => "NFKC",
            Form::Nfkd => "NFKD",
        }
    }

    fn quick_check(self, chars: impl Iterator<Item = char>) -> IsNormalized {
        match self {
            Form::Nfc => is_nfc_quick(chars),
            Form::Nfd => is_nfd_quick(chars),
            Form::Nfkc => is_nfkc_quick(chars),
            Form::Nfkd => is_nfkd_quick(chars),
        }
    }

    fn extend(self, out: &mut String, text: &str) {
        match self {
            Form::Nfc => out.extend(text.chars().nfc()),
            Form::Nfd => out.extend(text.chars().nfd()),
            Form::Nfkc => out.extend(text.chars().nfkc()),
            Form::Nfkd => out.extend(text.chars().nfkd()),
        }
    }

    /// True if text can be cut just before `c` and both sides normalized on
    /// their own: `c` is a starter (combining class 0) that decomposition
    /// leaves alone and, for NFC / NFKC, that never combines with the
    /// character before it. Characters below U+10000 are looked up in a
    /// bitmap (8 KiB per form, built on first use).
    fn has_boundary_before(self, c: char) -> bool {
        if c.is_ascii() {
            return true;
        }
        let cp = c as u32;
        if cp < 0x10000 {
            let bits = self.boundary_bitmap();
            return bits[(cp / 64) as usize] >> (cp % 64) & 1 == 1;
        }
        self.compute_boundary_before(c)
    }

    fn boundary_bitmap(self) -> &'static [u64; 1024] {
        static BITMAPS: [OnceLock<Box<[u64; 1024]>>; 4] = [const { OnceLock::new() }; 4];
        BITMAPS[self as usize].get_or_init(|| {
            let mut bits = Box::new([0u64; 1024]);
            for c in ('\0'..='\u{FFFF}').filter(|&c| self.compute_boundary_before(c)) {
                bits[(c as u32 / 64) as usize] |= 1 << (c as u32 % 64);
            }
            bits
        })
    }

    fn compute_boundary_before(self, c: char) -> bool {
        if canonical_combining_class(c) != 0 {
            return false;
        }
        let one = || iter::once(c);
        match self {
            Form::Nfd => is_nfd_quick(one()) == IsNormalized::Yes,
            Form::Nfkd => is_nfkd_quick(one()) == IsNormalized::Yes,
            Form::Nfc => {
                is_nfd_quick(one()) == IsNormalized::Yes && is_nfc_quick(one()) == IsNormalized::Yes
            }
            Form::Nfkc => {
                is_nfkd_quick(one()) == IsNormalized::Yes
                    && is_nfkc_quick(one()) == IsNormalized::Yes
            }
        }
    }
}

impl fmt::Display for Form {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Error for an unknown normalization form name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFormError(String);

impl fmt::Display for ParseFormError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown normalization form {:?} (expected NFC, NFD, NFKC or NFKD)",
            self.0
        )
    }
}

impl std::error::Error for ParseFormError {}

impl FromStr for Form {
    type Err = ParseFormError;

    /// `"NFC"`, `"NFD"`, `"NFKC"` or `"NFKD"`, in any letter case.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Form::ALL
            .into_iter()
            .find(|form| form.name().eq_ignore_ascii_case(s))
            .ok_or_else(|| ParseFormError(s.to_string()))
    }
}

/// Which steps [`normalize`] runs. The default does nothing.
///
/// ```ignore
/// let options = Options::nfkc().fold_whitespace().strip_invisible();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Options {
    /// Normalization form, or `None` to keep the text as it is.
    pub form: Option<Form>,
    /// Collapse whitespace runs into one U+0020 and trim both ends.
    pub fold_whitespace: bool,
    /// Delete the characters in [`INVISIBLE`].
    pub strip_invisible: bool,
}

impl Options {
    /// No step switched on.
    pub const fn new() -> Self {
        Options {
            form: None,
            fold_whitespace: false,
            strip_invisible: false,
        }
    }

    /// Normalize to `form` (or not at all with `None`).
    pub const fn with_form(mut self, form: Option<Form>) -> Self {
        self.form = form;
        self
    }

    pub const fn nfc() -> Self {
        Self::new().with_form(Some(Form::Nfc))
    }

    pub const fn nfd() -> Self {
        Self::new().with_form(Some(Form::Nfd))
    }

    pub const fn nfkc() -> Self {
        Self::new().with_form(Some(Form::Nfkc))
    }

    pub const fn nfkd() -> Self {
        Self::new().with_form(Some(Form::Nfkd))
    }

    /// Also fold whitespace (see [`fold_whitespace`]).
    pub const fn fold_whitespace(mut self) -> Self {
        self.fold_whitespace = true;
        self
    }

    /// Also remove invisible characters (see [`strip_invisible`]).
    pub const fn strip_invisible(mut self) -> Self {
        self.strip_invisible = true;
        self
    }
}

/// Runs the steps switched on in `options`: strip invisible, then normalize,
/// then fold whitespace. Borrows the input when nothing changes.
pub fn normalize(text: &str, options: Options) -> Cow<'_, str> {
    let mut out = Cow::Borrowed(text);
    if options.strip_invisible {
        out = then(out, strip_invisible);
    }
    if let Some(form) = options.form {
        out = then(out, |s| normalize_form(s, form));
    }
    if options.fold_whitespace {
        out = then(out, fold_whitespace);
    }
    out
}

/// Applies one step to the result of the previous one, keeping the borrow when possible.
fn then<'a>(prev: Cow<'a, str>, step: impl for<'b> Fn(&'b str) -> Cow<'b, str>) -> Cow<'a, str> {
    match prev {
        Cow::Borrowed(s) => step(s),
        Cow::Owned(s) => match step(&s) {
            Cow::Borrowed(_) => Cow::Owned(s),
            Cow::Owned(t) => Cow::Owned(t),
        },
    }
}

/// The text in normalization form `form`, the same as Python's
/// `unicodedata.normalize(form, text)`. Borrows the input when it is already
/// in that form.
///
/// Only the pieces that need it are normalized: the text is cut before every
/// character that can never interact with what comes before it (for example
/// any ASCII character), and pieces that pass the quick check are copied as
/// they are.
pub fn normalize_form(text: &str, form: Form) -> Cow<'_, str> {
    let mut pieces = Pieces {
        text,
        form,
        out: None,
        copied: 0,
        scratch: String::new(),
    };
    // The current piece is text[start..]; `single` while it is one boundary character
    let mut start = 0;
    let mut single = false;
    for (i, c) in text.char_indices() {
        if form.has_boundary_before(c) {
            if i > start && !single {
                pieces.check(start, i);
            }
            start = i;
            single = true;
        } else {
            single = false;
        }
    }
    if !single && start < text.len() {
        pieces.check(start, text.len());
    }
    match pieces.out {
        None => Cow::Borrowed(text),
        Some(mut out) => {
            out.push_str(&text[pieces.copied..]);
            Cow::Owned(out)
        }
    }
}

/// State of [`normalize_form`]: the output is only allocated once a piece needs to change.
struct Pieces<'a> {
    text: &'a str,
    form: Form,
    out: Option<String>,
    /// text[copied..] has not been written to `out` yet
    copied: usize,
    scratch: String,
}

impl Pieces<'_> {
    /// Normalizes text[start..end] (a piece that begins at a boundary, or at the start of the text) if needed.
    fn check(&mut self, start: usize, end: usize) {
        let piece = &self.text[start..end];
        let clean = match self.form.quick_check(piece.chars()) {
            IsNormalized::Yes => true,
            IsNormalized::No => false,
            IsNormalized::Maybe => {
                self.scratch.clear();
                self.form.extend(&mut self.scratch, piece);
                self.scratch == piece
            }
        };
        if !clean {
            let len = self.text.len();
            let out = self
                .out
                .get_or_insert_with(|| String::with_capacity(len + len / 8 + 8));
            out.push_str(&self.text[self.copied..start]);
            self.form.extend(out, piece);
            self.copied = end;
        }
    }
}

/// True if `text` is already in normalization form `form`, the same as
/// Python's `unicodedata.is_normalized(form, text)`.
pub fn is_normalized(text: &str, form: Form) -> bool {
    match form.quick_check(text.chars()) {
        IsNormalized::Yes => true,
        IsNormalized::No => false,
        IsNormalized::Maybe => matches!(normalize_form(text, form), Cow::Borrowed(_)),
    }
}

/// Whitespace as defined by Python's `str.isspace()` (Unicode 16.0.0), in
/// code point order. Rust's `char::is_whitespace` plus U+001C–U+001F.
pub const WHITESPACE: [char; 29] = [
    '\t', '\n', '\u{0B}', '\u{0C}', '\r', '\u{1C}', '\u{1D}', '\u{1E}', '\u{1F}', ' ', '\u{85}',
    '\u{A0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}',
    '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}', '\u{2028}', '\u{2029}', '\u{202F}',
    '\u{205F}', '\u{3000}',
];

/// True if Python's `str.isspace()` is true for `c` (one of [`WHITESPACE`]).
pub fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t'..='\r'
            | '\u{1C}'..=' '
            | '\u{85}'
            | '\u{A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
    )
}

/// Replaces every run of whitespace ([`is_whitespace`]) with one U+0020 SPACE
/// and removes whitespace at both ends, like Python's `" ".join(text.split())`.
/// Borrows the input when it is already folded.
pub fn fold_whitespace(text: &str) -> Cow<'_, str> {
    // Already folded: only single U+0020 between non-space characters
    let mut after_space = true;
    let folded = text.chars().all(|c| {
        if is_whitespace(c) {
            let ok = c == ' ' && !after_space;
            after_space = true;
            ok
        } else {
            after_space = false;
            true
        }
    }) && (text.is_empty() || !after_space);
    if folded {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    for word in text.split(is_whitespace).filter(|w| !w.is_empty()) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    Cow::Owned(out)
}

/// The invisible characters removed by [`strip_invisible`], as inclusive
/// ranges in code point order: every character that is both
/// `General_Category = Cf` and `Default_Ignorable_Code_Point` in Unicode 16.0.0.
pub const INVISIBLE: [(char, char); 12] = [
    ('\u{AD}', '\u{AD}'),       // SOFT HYPHEN
    ('\u{61C}', '\u{61C}'),     // ARABIC LETTER MARK
    ('\u{180E}', '\u{180E}'),   // MONGOLIAN VOWEL SEPARATOR
    ('\u{200B}', '\u{200F}'),   // ZERO WIDTH SPACE, ZWNJ, ZWJ, LRM, RLM
    ('\u{202A}', '\u{202E}'),   // bidi embeddings and overrides
    ('\u{2060}', '\u{2064}'),   // WORD JOINER, invisible math operators
    ('\u{2066}', '\u{206F}'),   // bidi isolates, deprecated format characters
    ('\u{FEFF}', '\u{FEFF}'),   // BYTE ORDER MARK (ZERO WIDTH NO-BREAK SPACE)
    ('\u{1BCA0}', '\u{1BCA3}'), // shorthand format controls
    ('\u{1D173}', '\u{1D17A}'), // musical symbol beam, tie, slur and phrase controls
    ('\u{E0001}', '\u{E0001}'), // LANGUAGE TAG
    ('\u{E0020}', '\u{E007F}'), // tag characters
];

/// True if `c` is one of the [`INVISIBLE`] characters.
pub fn is_invisible(c: char) -> bool {
    // Same ranges as INVISIBLE, written out so that the compiler can build a decision tree
    matches!(
        c,
        '\u{AD}'
            | '\u{61C}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0001}'
            | '\u{E0020}'..='\u{E007F}'
    )
}

/// Deletes every [`INVISIBLE`] character. Borrows the input when there is none.
pub fn strip_invisible(text: &str) -> Cow<'_, str> {
    let Some(first) = text.find(is_invisible) else {
        return Cow::Borrowed(text);
    };
    // Copy the runs between invisible characters
    let mut out = String::with_capacity(text.len());
    let mut kept = 0;
    for (i, c) in text[first..].char_indices() {
        if is_invisible(c) {
            out.push_str(&text[kept..first + i]);
            kept = first + i + c.len_utf8();
        }
    }
    out.push_str(&text[kept..]);
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_bitmap_matches_the_direct_check() {
        for form in Form::ALL {
            for c in '\0'..='\u{FFFF}' {
                assert_eq!(
                    form.has_boundary_before(c),
                    form.compute_boundary_before(c),
                    "{form} {c:?}"
                );
            }
        }
    }
}
