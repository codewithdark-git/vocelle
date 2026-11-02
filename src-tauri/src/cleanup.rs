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

/// Hesitation sounds that never carry meaning. Words like "like" or "so" are
/// left alone because they often do.
const FILLERS: &[&str] = &[
    "um", "umm", "uh", "uhh", "uh-huh", "er", "erm", "ah", "ahh", "hmm", "hm", "mm", "mhm",
];

pub fn clean(text: &str, level: Level, replacements: &[Replacement]) -> String {
    let replaced = apply_replacements(text.trim(), replacements);
    if level == Level::Off {
        return replaced;
    }
    // Line breaks (e.g. from a dictionary rule) are kept; each line is cleaned on its own.
    replaced
        .split('\n')
        .map(|line| clean_line(line, level))
        .collect::<Vec<_>>()
        .join("\n")
}

fn clean_line(line: &str, level: Level) -> String {
    let mut out = remove_fillers(line);
    if level == Level::Full {
        out = remove_repeated_words(&out);
        out = fix_standalone_i(&out);
    }
    out = fix_spacing(&out);
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

fn remove_fillers(text: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    for token in text.split_whitespace() {
        let (_, word, trail) = split_word(token);
        if !FILLERS.contains(&word.to_lowercase().as_str()) {
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

fn remove_repeated_words(text: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for token in text.split_whitespace() {
        if let Some(prev) = out.last() {
            let (_, a, a_trail) = split_word(prev);
            let (_, b, _) = split_word(token);
            // "the the" is a stutter; "that, that" (with a comma) is deliberate.
            // Single letters other than "I" are skipped ("a a" is rare, "x x" may be math).
            let stutter =
                a.eq_ignore_ascii_case(b) && a_trail.is_empty() && (a.len() > 1 || a == "I");
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

/// Collapses runs of whitespace and removes spaces before punctuation.
fn fix_spacing(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::with_capacity(collapsed.len());
    for c in collapsed.chars() {
        if matches!(c, ',' | '.' | '?' | '!' | ';' | ':') && out.ends_with(' ') {
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

fn capitalize_sentences(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut capitalize = true;
    for c in text.chars() {
        if capitalize && c.is_alphabetic() {
            out.extend(c.to_uppercase());
            capitalize = false;
            continue;
        }
        if matches!(c, '.' | '?' | '!') {
            capitalize = true;
        } else if !c.is_whitespace() && !matches!(c, '"' | '\'' | '(' | ')') {
            // Digits and other characters end the "start of sentence" state,
            // so "3.5" stays "3.5" and doesn't capitalize what follows.
            capitalize = false;
        }
        out.push(c);
    }
    out
}

fn apply_replacements(text: &str, replacements: &[Replacement]) -> String {
    let mut out = text.to_owned();
    for r in replacements {
        let from = r.from.trim();
        if from.is_empty() {
            continue;
        }
        // `to` isn't trimmed, so a rule can produce a line break or trailing space.
        out = replace_whole_word(&out, from, &r.to);
    }
    out
}

/// Replaces `from` (one or more words) case-insensitively, only at word boundaries.
fn replace_whole_word(text: &str, from: &str, to: &str) -> String {
    let lower_text = text.to_lowercase();
    let lower_from = from.to_lowercase();
    // Lowercasing can change byte lengths for some scripts; fall back to exact matching then.
    if lower_text.len() != text.len() || lower_from.len() != from.len() {
        return text.replace(from, to);
    }
    let is_word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '\'');
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while let Some(pos) = lower_text[i..].find(&lower_from) {
        let start = i + pos;
        let end = start + from.len();
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        out.push_str(&text[i..start]);
        if is_word(before) || is_word(after) {
            out.push_str(&text[start..end]);
        } else {
            out.push_str(to);
        }
        i = end;
    }
    out.push_str(&text[i..]);
    out
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
        clean(s, Level::Light, &[])
    }

    fn full(s: &str) -> String {
        clean(s, Level::Full, &[])
    }

    #[test]
    fn off_keeps_text_as_is() {
        assert_eq!(
            clean("  um so  i think ", Level::Off, &[]),
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
            clean("open vocel and review the PR", Level::Off, &r),
            "open Vocelle and review the PR"
        );
        assert_eq!(
            clean("open Vocel. approve pr.", Level::Off, &r),
            "open Vocelle. approve PR."
        );
        // Not inside other words.
        assert_eq!(
            clean("vocelling and prize", Level::Off, &r),
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
            clean("first new line second", Level::Off, &r),
            "first \n second"
        );
        assert_eq!(
            clean("first new line um second", Level::Full, &r),
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
        assert_eq!(clean("go to İstanbul", Level::Off, &r), "go to Istanbul");
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
