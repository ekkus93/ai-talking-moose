//! P0-only Kitten Mini tokenizer.
//!
//! Vocabulary and tokenization are derived from the Apache-2.0 `kittentts-rs`
//! 0.4.1 port, but the framing intentionally follows the official KittenTTS
//! Mini 0.8 Python implementation exactly: prepend 0 and append 10, 0.

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

const PAD: char = '$';
const PUNCTUATION: &str = ";:,.!?¡¿—…\u{201C}«»\u{201D}\" ";
const LETTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const IPA_LETTERS: &str =
    "ɑɐɒæɓʙβɔɕçɗɖðʤəɘɚɛɜɝɞɟʄɡɠɢʛɦɧħɥʜɨɪʝɭɬɫɮʟɱɯɰŋɳɲɴøɵɸθœɶʘɹɺɾɻʀʁɽʂʃʈʧʉʊʋⱱʌɣɤʍχʎʏʑʐʒʔʡʕʢǀǁǂǃˈˌːˑʼʴʰʱʲʷˠˤ˞↓↑→↗↘\u{2019}\u{0329}\u{2018}ᵻ";

static VOCAB: Lazy<HashMap<char, i64>> = Lazy::new(|| {
    std::iter::once(PAD)
        .chain(PUNCTUATION.chars())
        .chain(LETTERS.chars())
        .chain(IPA_LETTERS.chars())
        .enumerate()
        .map(|(index, symbol)| (symbol, index as i64))
        .collect()
});

static TOKEN_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\w+|[^\w\s]").unwrap());

pub fn ipa_to_ids(ipa: &str) -> Vec<i64> {
    let tokenized = TOKEN_RE
        .find_iter(ipa)
        .map(|item| item.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    let mut ids = Vec::with_capacity(tokenized.chars().count() + 3);
    ids.push(0);
    ids.extend(tokenized.chars().filter_map(|symbol| VOCAB.get(&symbol).copied()));
    ids.push(10);
    ids.push(0);
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing_matches_official_kitten_v08() {
        let ids = ipa_to_ids("hɛloʊ");
        assert_eq!(ids.first(), Some(&0));
        assert_eq!(&ids[ids.len() - 2..], &[10, 0]);
    }
}
