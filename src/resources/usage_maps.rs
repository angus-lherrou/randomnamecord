use lazy_static::lazy_static;
use regex::{Regex, RegexSet};

// const _USAGE_MAP: [(&str, &str); 2] = [
//     ("^gmc-.+$", "ger"),
//     ("^(.+)-(?:myth|bibl|medi)$", "$1"),
// ];

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
}
//
// fn fun() {
//     let USAGE_MAP: [(Regex, &str); 2] = [
//         (Regex::new("^gmc-.+$").unwrap(), "ger"),
//         (Regex::new("^(.+)-(?:myth|bibl|medi)$").unwrap(), "$1"),
//     ];
//     let foo = RegexSet::new(USAGE_MAP
//         .iter()
//         .map(|(pat, _)| pat)
//         .collect::<Vec<&Regex>>()).unwrap();
//
// }
