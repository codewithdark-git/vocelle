//! Turns raw transcripts into text that reads as if it was typed.
//!
//! Everything here is deterministic, rule-based and local, so it runs in
//! microseconds and never changes the meaning of what was said.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Paste exactly what the speech model produced.
    Off,
    /// Remove filler sounds and fix spacing.
    #[default]
    Light,
    /// Light, plus repeated words, sentence capitalization and a standalone "i".
    Full,
}

/// A whole-word, case-insensitive replacement from the personal dictionary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replacement {
    pub from: String,
    pub to: String,
}

/// The language-specific rules for one dictation, picked from the language
/// Whisper used. Languages without a table get only the language-neutral
/// steps (spacing, capitals, stutters), so cleanup never deletes a real word
/// it doesn't know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lang {
    English,
    German,
    French,
    Dutch,
    Italian,
    Other,
}

impl Lang {
    /// From a Whisper language code such as "en" or "de". "auto", an empty
    /// code or an unknown one gives `Other`.
    fn from_code(code: &str) -> Self {
        match base_code(code).as_str() {
            "en" => Lang::English,
            "de" => Lang::German,
            "fr" => Lang::French,
            "nl" => Lang::Dutch,
            "it" => Lang::Italian,
            _ => Lang::Other,
        }
    }

    /// Hesitation sounds that never carry meaning in this language. Words
    /// like "like" or "so" are left alone because they often do, and so are
    /// sounds that are real words elsewhere: German "er" (he) and "um" (at),
    /// Dutch "er" (there), Portuguese "um" (a, one).
    fn fillers(self) -> &'static [&'static str] {
        match self {
            Lang::English => &[
                "um", "umm", "uh", "uhh", "uh-huh", "er", "erm", "ah", "ahh", "hmm", "hm", "mm",
                "mhm",
            ],
            Lang::German => &["äh", "ähm", "öh", "öhm", "hmm", "hm", "mhm"],
            Lang::French => &["euh", "heu", "hmm", "hm"],
            Lang::Dutch => &["eh", "ehm", "uh", "uhm", "hmm", "hm"],
            Lang::Italian => &["ehm", "hmm", "hm"],
            Lang::Other => &[],
        }
    }

    /// Words that are often said twice on purpose ("I had had enough",
    /// German "die Frau, die die Zeitung liest"), so they're never treated as
    /// a stutter.
    fn deliberate_doubles(self) -> &'static [&'static str] {
        match self {
            Lang::English => &["had", "that"],
            Lang::German => &["der", "die", "das"],
            Lang::Dutch => &["dat", "die"],
            _ => &[],
        }
    }
}

/// "en" from "en", "EN" or "en-US".
fn base_code(code: &str) -> String {
    code.trim()
        .split(['-', '_'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Whether the language capitalizes nouns, so a capital in the middle of a
/// sentence is part of the word and must be kept.
pub fn capitalizes_nouns(language: &str) -> bool {
    matches!(base_code(language).as_str(), "de" | "lb")
}

/// Words ending in a full stop that usually don't end a sentence, so Full
/// doesn't capitalize the word after them. Compared in lowercase.
const ABBREVIATIONS: &[&str] = &[
    "e.g.", "i.e.", "etc.", "vs.", "cf.", "approx.", "mr.", "mrs.", "ms.", "dr.", "prof.", "z.b.",
    "d.h.", "bzw.", "usw.", "ca.",
];

/// Cleans a transcript. `language` is the code Whisper transcribed in, such
/// as "en" or "de"; "auto" or an unknown code skips the rules that only
/// make sense in one language (fillers, "i" → "I").
pub fn clean(text: &str, level: Level, language: &str, replacements: &[Replacement]) -> String {
    let replaced = apply_replacements(text.trim(), replacements);
    if level == Level::Off {
        return replaced;
    }
    let lang = Lang::from_code(language);
    // Line breaks (e.g. from a dictionary rule) are kept; each line is cleaned on its own.
    replaced
        .split('\n')
        .map(|line| clean_line(line, level, lang))
        .collect::<Vec<_>>()
        .join("\n")
}

fn clean_line(line: &str, level: Level, lang: Lang) -> String {
    let mut out = remove_fillers(line, lang);
    if level == Level::Full {
        out = remove_repeated_words(&out, lang);
        if lang == Lang::English {
            out = fix_standalone_i(&out);
        }
    }
    out = fix_spacing(&out, lang);
    if level == Level::Full {
        capitalize_sentences(&out)
    } else {
        capitalize_first(&out)
    }
}

/// Splits into words while keeping attached punctuation, e.g. `"um,"` → `("", "um", ",")`.
fn split_word(token: &str) -> (&str, &str, &str) {
    let start = token
        .find(|c: char| c.is_alphanumeric())
        .unwrap_or(token.len());
    let end = token
        .rfind(|c: char| c.is_alphanumeric())
        .map_or(start, |i| {
            i + token[i..].chars().next().map_or(1, char::len_utf8)
        });
    (&token[..start], &token[start..end], &token[end..])
}

fn remove_fillers(text: &str, lang: Lang) -> String {
    let fillers = lang.fillers();
    if fillers.is_empty() {
        return text.to_owned();
    }
    let mut words: Vec<String> = Vec::new();
    for token in text.split_whitespace() {
        let (_, word, trail) = split_word(token);
        // After a number it's a unit, not a sound: "5 mm".
        let after_number = words
            .last()
            .is_some_and(|w| w.ends_with(|c: char| c.is_ascii_digit()));
        if after_number || !fillers.contains(&word.to_lowercase().as_str()) {
            words.push(token.to_owned());
            continue;
        }
        if let Some(last) = words.last_mut() {
            // Keep sentence-ending punctuation from the filler ("today, um." → "today.").
            let end: String = trail
                .chars()
                .filter(|c| matches!(c, '.' | '?' | '!'))
                .collect();
            if !end.is_empty() {
                *last = last.trim_end_matches([',', ';', ':']).to_owned() + &end;
            } else if trail.contains(',') {
                // A filler set off by commas: "we should, um, ship" → "we should ship".
                *last = last.trim_end_matches(',').to_owned();
            }
        }
    }
    // A filler at the very start can leave a dangling comma: "Um, so" → ", so".
    words
        .join(" ")
        .trim_start_matches([',', ';', ' '])
        .to_owned()
}

fn remove_repeated_words(text: &str, lang: Lang) -> String {
    let mut out: Vec<&str> = Vec::new();
    for token in text.split_whitespace() {
        if let Some(prev) = out.last() {
            let (_, a, a_trail) = split_word(prev);
            let (_, b, _) = split_word(token);
            let a_lower = a.to_lowercase();
            // "the the" is a stutter; "that, that" (with a comma) is deliberate.
            // Single letters other than "I" are skipped ("a a" is rare, "x x" may be math).
            let stutter = a_lower == b.to_lowercase()
                && a_trail.is_empty()
                && (a.chars().count() > 1 || a == "I")
                && !lang.deliberate_doubles().contains(&a_lower.as_str());
            if stutter {
                out.pop();
            }
        }
        out.push(token);
    }
    out.join(" ")
}

fn fix_standalone_i(text: &str) -> String {
    text.split_whitespace()
        .map(|token| {
            let (lead, word, trail) = split_word(token);
            match word {
                "i" => format!("{lead}I{trail}"),
                "i'm" | "i'll" | "i've" | "i'd" => format!("{lead}I{}{trail}", &word[1..]),
                _ => token.to_owned(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Collapses runs of whitespace and removes spaces before punctuation. French
/// keeps its space before "?", "!", ";" and ":".
fn fix_spacing(text: &str, lang: Lang) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::with_capacity(collapsed.len());
    for c in collapsed.chars() {
        let tight = match lang {
            Lang::French => matches!(c, ',' | '.'),
            _ => matches!(c, ',' | '.' | '?' | '!' | ';' | ':'),
        };
        if tight && out.ends_with(' ') {
            out.pop();
        }
        // Doubled punctuation left behind by removed words: ",," or ",."
        if c == ',' && out.ends_with([',', '.', '?', '!']) {
            continue;
        }
        if matches!(c, '.' | '?' | '!') && out.ends_with(',') {
            out.pop();
        }
        out.push(c);
    }
    out.trim().to_owned()
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Capitalizes the first letter of each sentence. A sentence ends at ".",
/// "?" or "!" followed by whitespace, so "example.com", "main.rs",
/// "me@example.com" and "3.5" are left alone, and so is the word after an
/// abbreviation such as "e.g.".
fn capitalize_sentences(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    // At the start of a sentence: the next letter is capitalized.
    let mut capitalize = true;
    // Just after sentence-ending punctuation (and any closing quotes).
    let mut ended = false;
    for c in text.chars() {
        if capitalize && c.is_alphabetic() {
            out.extend(c.to_uppercase());
            capitalize = false;
            continue;
        }
        out.push(c);
        if matches!(c, '.' | '?' | '!' | '…') {
            ended = !(c == '.' && ends_with_abbreviation(&out));
        } else if c.is_whitespace() {
            capitalize |= ended;
            ended = false;
        } else if ended && matches!(c, '"' | '\'' | ')' | '”' | '’' | '»') {
            // A closing quote after the full stop: the sentence still ended.
        } else if capitalize && matches!(c, '"' | '\'' | '(' | '“' | '‘' | '«' | '¿' | '¡') {
            // An opening quote before the first word.
        } else {
            // Digits and other characters end the "start of sentence" state,
            // so "3 apples" and "3.5" stay as they are.
            capitalize = false;
            ended = false;
        }
    }
    out
}

/// Whether the last word of `text` (ending in ".") is a known abbreviation.
fn ends_with_abbreviation(text: &str) -> bool {
    let word = text
        .rsplit(char::is_whitespace)
        .next()
        .unwrap_or_default()
        .trim_start_matches(['"', '\'', '(', '“', '‘', '«'])
        .to_lowercase();
    ABBREVIATIONS.contains(&word.as_str())
}

/// Applies the personal dictionary in one pass: at each word start the
/// longest matching rule wins, so one rule's output is never replaced again
/// by another. Matching is whole-word and case-insensitive, and works on
/// characters, so letters that change length when lowercased ("İ") are safe.
fn apply_replacements(text: &str, replacements: &[Replacement]) -> String {
    let mut rules: Vec<(Vec<char>, &str)> = replacements
        .iter()
        .filter_map(|r| {
            let from = r.from.trim();
            // `to` isn't trimmed, so a rule can produce a line break or trailing space.
            (!from.is_empty()).then(|| (from.chars().collect(), r.to.as_str()))
        })
        .collect();
    if rules.is_empty() {
        return text.to_owned();
    }
    // Stable, so rules of the same length keep the order they were added in.
    rules.sort_by_key(|(from, _)| std::cmp::Reverse(from.len()));

    let chars: Vec<char> = text.chars().collect();
    let is_word = |c: Option<&char>| c.is_some_and(|&c| c.is_alphanumeric() || c == '\'');
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let at_word_start = i == 0 || !is_word(chars.get(i - 1));
        let rule = rules.iter().find(|(from, _)| {
            let end = i + from.len();
            at_word_start
                && end <= chars.len()
                && same_letters(&chars[i..end], from)
                && !is_word(chars.get(end))
        });
        match rule {
            Some((from, to)) => {
                out.push_str(to);
                i += from.len();
            }
            None => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }
    out
}

/// Case-insensitive comparison of two equally long runs of characters.
fn same_letters(a: &[char], b: &[char]) -> bool {
    a.iter()
        .zip(b)
        .all(|(x, y)| x == y || x.to_lowercase().eq(y.to_lowercase()))
}

/// Builds a Whisper prompt from dictionary words, which biases recognition
/// toward their spelling. Kept short: long prompts slow transcription.
pub fn vocabulary_prompt(words: &[String]) -> Option<String> {
    const MAX_CHARS: usize = 600;
    let mut prompt = String::new();
    for w in words.iter().map(|w| w.trim()).filter(|w| !w.is_empty()) {
        if prompt.len() + w.len() + 2 > MAX_CHARS {
            break;
        }
        if !prompt.is_empty() {
            prompt.push_str(", ");
        }
        prompt.push_str(w);
    }
    (!prompt.is_empty()).then(|| format!("{prompt}."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn light(s: &str) -> String {
        clean(s, Level::Light, "en", &[])
    }

    fn full(s: &str) -> String {
        clean(s, Level::Full, "en", &[])
    }

    #[test]
    fn off_keeps_text_as_is() {
        assert_eq!(
            clean("  um so  i think ", Level::Off, "en", &[]),
            "um so  i think"
        );
    }

    #[test]
    fn removes_fillers_with_their_commas() {
        assert_eq!(
            light("Um, so I think, uh, we should go."),
            "So I think we should go."
        );
        assert_eq!(
            light("We should, um, ship it today."),
            "We should ship it today."
        );
        assert_eq!(light("Hmm. Okay."), "Okay.");
    }

    #[test]
    fn filler_at_end_keeps_the_full_stop() {
        assert_eq!(light("Let's meet tomorrow, um."), "Let's meet tomorrow.");
        assert_eq!(light("Is it ready, uh?"), "Is it ready?");
    }

    #[test]
    fn keeps_words_that_only_look_like_fillers() {
        assert_eq!(
            light("The umbrella is in the hmmer room."),
            "The umbrella is in the hmmer room."
        );
        assert_eq!(light("I like it, so let's go."), "I like it, so let's go.");
    }

    #[test]
    fn fixes_spacing_around_punctuation() {
        assert_eq!(light("hello  ,  world !"), "Hello, world!");
    }

    #[test]
    fn light_capitalizes_only_the_first_letter() {
        assert_eq!(light("send it. then call"), "Send it. then call");
    }

    #[test]
    fn full_capitalizes_every_sentence() {
        assert_eq!(
            full("send it. then call! ok? yes"),
            "Send it. Then call! Ok? Yes"
        );
        assert_eq!(full("version 3.5 is out"), "Version 3.5 is out");
    }

    #[test]
    fn full_removes_stuttered_words() {
        assert_eq!(full("I think the the plan works"), "I think the plan works");
        assert_eq!(full("I I agree"), "I agree");
        // A comma means it was deliberate: "that, that is the point".
        assert_eq!(full("that, that is the point"), "That, that is the point");
    }

    #[test]
    fn full_fixes_standalone_i() {
        assert_eq!(full("yes i'm sure i can"), "Yes I'm sure I can");
        assert_eq!(full("this is it"), "This is it");
    }

    #[test]
    fn replacements_are_whole_word_and_case_insensitive() {
        let r = vec![
            Replacement {
                from: "vocel".into(),
                to: "Vocelle".into(),
            },
            Replacement {
                from: "pr".into(),
                to: "PR".into(),
            },
        ];
        assert_eq!(
            clean("open vocel and review the PR", Level::Off, "en", &r),
            "open Vocelle and review the PR"
        );
        assert_eq!(
            clean("open Vocel. approve pr.", Level::Off, "en", &r),
            "open Vocelle. approve PR."
        );
        // Not inside other words.
        assert_eq!(
            clean("vocelling and prize", Level::Off, "en", &r),
            "vocelling and prize"
        );
    }

    #[test]
    fn multi_word_replacements() {
        let r = vec![Replacement {
            from: "new line".into(),
            to: "\n".into(),
        }];
        assert_eq!(
            clean("first new line second", Level::Off, "en", &r),
            "first \n second"
        );
        assert_eq!(
            clean("first new line um second", Level::Full, "en", &r),
            "First\nSecond"
        );
    }

    #[test]
    fn empty_and_unicode_input_are_safe() {
        assert_eq!(full(""), "");
        assert_eq!(full("   "), "");
        assert_eq!(full("über straße, um, café"), "Über straße café");
        let r = vec![Replacement {
            from: "İstanbul".into(),
            to: "Istanbul".into(),
        }];
        assert_eq!(
            clean("go to İstanbul", Level::Off, "en", &r),
            "go to Istanbul"
        );
    }

    /// Golden regression corpus: (language, level, what Whisper heard,
    /// what gets typed). Every rule change has to keep these.
    const CORPUS: &[(&str, Level, &str, &str)] = &[
        // English fillers and stutters.
        (
            "en",
            Level::Light,
            "Um, so I think, uh, we should go.",
            "So I think we should go.",
        ),
        (
            "en",
            Level::Light,
            "Let's meet tomorrow, um.",
            "Let's meet tomorrow.",
        ),
        (
            "en",
            Level::Light,
            "The umbrella is in the hmmer room.",
            "The umbrella is in the hmmer room.",
        ),
        (
            "en",
            Level::Light,
            "The bolt is 5 mm wide.",
            "The bolt is 5 mm wide.",
        ),
        (
            "en",
            Level::Full,
            "um so i think the the plan works, uh, for friday. i'll send it",
            "So I think the plan works for friday. I'll send it",
        ),
        ("en", Level::Full, "I I agree", "I agree"),
        (
            "en",
            Level::Full,
            "she had had enough",
            "She had had enough",
        ),
        (
            "en",
            Level::Full,
            "he said that that was fine",
            "He said that that was fine",
        ),
        // English rules don't apply to other languages.
        (
            "de",
            Level::Light,
            "Er kommt um 3 Uhr.",
            "Er kommt um 3 Uhr.",
        ),
        (
            "de",
            Level::Full,
            "Äh, er kommt um 3 Uhr, ähm, oder später.",
            "Er kommt um 3 Uhr oder später.",
        ),
        (
            "de",
            Level::Full,
            "die Frau, die die Zeitung liest",
            "Die Frau, die die Zeitung liest",
        ),
        (
            "de",
            Level::Full,
            "ich weiß, i bin da",
            "Ich weiß, i bin da",
        ),
        ("pt", Level::Light, "Eu quero um café.", "Eu quero um café."),
        (
            "pt",
            Level::Full,
            "é um bom dia. ah sim",
            "É um bom dia. Ah sim",
        ),
        (
            "nl",
            Level::Light,
            "Er is, eh, niemand thuis.",
            "Er is niemand thuis.",
        ),
        ("nl", Level::Light, "Is er nog koffie?", "Is er nog koffie?"),
        (
            "fr",
            Level::Light,
            "Euh, tu viens ce soir ?",
            "Tu viens ce soir ?",
        ),
        ("es", Level::Light, "um, hola", "Um, hola"),
        // Unknown or undetected language: nothing language-specific.
        ("auto", Level::Light, "um yes", "Um yes"),
        ("", Level::Full, "um i agree", "Um i agree"),
        // Region codes count as their language.
        ("en-US", Level::Light, "uh, ok", "Ok"),
        // URLs, domains, emails, file names and decimals keep their case.
        (
            "en",
            Level::Full,
            "go to example.com. then sign in",
            "Go to example.com. Then sign in",
        ),
        (
            "en",
            Level::Full,
            "open https://docs.example.org/guide.html now",
            "Open https://docs.example.org/guide.html now",
        ),
        (
            "en",
            Level::Full,
            "mail me at jane.doe@example.co.uk today",
            "Mail me at jane.doe@example.co.uk today",
        ),
        (
            "en",
            Level::Full,
            "edit main.rs and lib.rs",
            "Edit main.rs and lib.rs",
        ),
        (
            "en",
            Level::Full,
            "version 3.5 is out. ship it",
            "Version 3.5 is out. Ship it",
        ),
        (
            "en",
            Level::Full,
            "it costs 2.50 dollars",
            "It costs 2.50 dollars",
        ),
        // Abbreviations don't end a sentence.
        (
            "en",
            Level::Full,
            "bring fruit, e.g. apples. and bread",
            "Bring fruit, e.g. apples. And bread",
        ),
        (
            "en",
            Level::Full,
            "use a tool, i.e. a hammer",
            "Use a tool, i.e. a hammer",
        ),
        (
            "en",
            Level::Full,
            "pens, paper etc. are here",
            "Pens, paper etc. are here",
        ),
        ("en", Level::Full, "ask dr. smith", "Ask dr. smith"),
        (
            "de",
            Level::Full,
            "Obst, z.B. Äpfel. danach Brot",
            "Obst, z.B. Äpfel. Danach Brot",
        ),
        // Sentence ends still capitalize, also after quotes and ellipses.
        (
            "en",
            Level::Full,
            "send it. then call! ok? yes",
            "Send it. Then call! Ok? Yes",
        ),
        (
            "en",
            Level::Full,
            "he said \"go.\" then left",
            "He said \"go.\" Then left",
        ),
        ("en", Level::Full, "wait... then go", "Wait... Then go"),
        (
            "en",
            Level::Full,
            "über straße, um, café. ändern",
            "Über straße café. Ändern",
        ),
        // French keeps the space before "?" and "!".
        (
            "fr",
            Level::Full,
            "c'est vrai ! d'accord",
            "C'est vrai ! D'accord",
        ),
        ("en", Level::Light, "hello  ,  world !", "Hello, world!"),
    ];

    #[test]
    fn golden_corpus() {
        let failures: Vec<String> = CORPUS
            .iter()
            .filter_map(|&(lang, level, input, expected)| {
                let got = clean(input, level, lang, &[]);
                (got != expected).then(|| {
                    format!(
                        "[{lang} {level:?}] {input:?}
  expected {expected:?}
  got      {got:?}"
                    )
                })
            })
            .collect();
        assert!(
            failures.is_empty(),
            "
{}",
            failures.join(
                "
"
            )
        );
    }

    #[test]
    fn unicode_replacements_are_whole_word() {
        let rule = |from: &str, to: &str| Replacement {
            from: from.into(),
            to: to.into(),
        };
        // "İ" grows when lowercased; matching used to fall back to plain
        // substring replacement then, which changed "book" to "boOK".
        let r = vec![rule("ok", "OK")];
        assert_eq!(
            clean("İzmir book Ok ok", Level::Off, "en", &r),
            "İzmir book OK OK"
        );
        let r = vec![rule("über", "Über"), rule("straße", "Straße")];
        assert_eq!(
            clean("ÜBER die straße, überall", Level::Off, "de", &r),
            "Über die Straße, überall"
        );
        let r = vec![rule("кот", "Кот")];
        assert_eq!(
            clean("мой кот, котёнок", Level::Off, "", &r),
            "мой Кот, котёнок"
        );
        let r = vec![rule("東京", "Tokyo")];
        assert_eq!(clean("東京 に", Level::Off, "", &r), "Tokyo に");
    }

    #[test]
    fn replacements_run_once_with_the_longest_rule_first() {
        let rule = |from: &str, to: &str| Replacement {
            from: from.into(),
            to: to.into(),
        };
        let r = vec![
            rule("new", "NEW"),
            rule(
                "new line", "
",
            ),
        ];
        assert_eq!(
            clean("a new line new", Level::Off, "en", &r),
            "a 
 NEW"
        );
        // One rule's output isn't replaced again by the next.
        let r = vec![rule("a", "b"), rule("b", "c")];
        assert_eq!(clean("a b", Level::Off, "en", &r), "b c");
    }

    #[test]
    fn german_nouns_are_capitalized() {
        assert!(capitalizes_nouns("de"));
        assert!(capitalizes_nouns("DE-at"));
        assert!(!capitalizes_nouns("en"));
        assert!(!capitalizes_nouns("auto"));
    }

    #[test]
    fn vocabulary_prompt_joins_and_limits() {
        assert_eq!(vocabulary_prompt(&[]), None);
        assert_eq!(
            vocabulary_prompt(&["Vocelle".into(), " ".into(), "Tauri".into()]),
            Some("Vocelle, Tauri.".into())
        );
        let many: Vec<String> = (0..500).map(|i| format!("word{i}")).collect();
        assert!(vocabulary_prompt(&many).unwrap().len() <= 601);
    }
}
