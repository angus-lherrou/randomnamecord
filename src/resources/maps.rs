use aho_corasick::AhoCorasick;
use lazy_static::lazy_static;
use regex::{Regex, RegexSet};
use serde::Deserialize;
use serde_json::from_str;

#[derive(Deserialize, Debug)]
struct NormPair {
    target: String,
    code: String,
}

const LUT: &str = include_str!("lut.json");

lazy_static! {
    pub(crate) static ref USAGE_MAP: [(Regex, &'static str); 2] = [
        (Regex::new("^gmc-.+$").unwrap(), "ger"),
        (Regex::new("^(.+)-(?:myth|bibl|medi)$").unwrap(), "$1"),
    ];
    pub(crate) static ref USAGE_REGEX: RegexSet = RegexSet::new(
        USAGE_MAP
            .iter()
            .map(|(pat, _)| pat.as_str())
            .collect::<Vec<_>>()
    )
    .unwrap();
    static ref NMAP_JSON: Vec<NormPair> = from_str(LUT).unwrap();
    pub(crate) static ref NORM_TARGETS: Vec<&'static str> = NMAP_JSON
        .iter()
        .map(|NormPair { target, .. }| { target.as_str() })
        .collect();
    pub(crate) static ref NORM_CODES: Vec<&'static str> = NMAP_JSON
        .iter()
        .map(|NormPair { code, .. }| { code.as_str() })
        .collect();
    pub(crate) static ref NORM_AC: AhoCorasick = AhoCorasick::new(NORM_TARGETS.iter());
}
