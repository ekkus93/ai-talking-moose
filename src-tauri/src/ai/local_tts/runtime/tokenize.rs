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

static TOKEN_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\w+|[^\w\s]").expect("static regex"));

pub(super) fn ipa_to_ids(ipa: &str) -> Vec<i64> {
    let tokenized = TOKEN_RE
        .find_iter(ipa)
        .map(|item| item.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    let mut ids = Vec::with_capacity(tokenized.chars().count() + 3);
    ids.push(0);
    ids.extend(
        tokenized
            .chars()
            .filter_map(|symbol| VOCAB.get(&symbol).copied()),
    );
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
