extern crate regex;

use regex::Regex;
use std;

fn starts_name(index: i32, full_name: &str) -> bool {
    if index <= 0 {
        return true;
    }
    let full_name = full_name.as_bytes();
    let prev: char = full_name[(index - 1) as usize].into();
    let curr: char = full_name[index as usize].into();
    prev == '-' || prev == '_' || curr.is_uppercase()
}

fn score_name(name: &String, pattern: &Regex) -> Option<i32> {
    if !pattern.is_match(&name) {
        return None;
    }

    let matches = pattern.captures(&name).unwrap();
    let mut start_idxs: Vec<_> = vec![-1];

    let m_start_idxs: Vec<_> = matches
        .iter()
        .skip(1)
        .map(|m| m.unwrap().start() as i32)
        .collect();
    start_idxs.extend(m_start_idxs);

    let mut score = 0;

    for idx in 0..start_idxs.len() - 1 {
        let i1: i32 = start_idxs[idx];
        let i2: i32 = start_idxs[idx + 1];
        if starts_name(i1 as i32, name) && starts_name(i2 as i32, name) {
            continue;
        }
        score = score + (i2 - (i1 + 1));
    }

    Some(score)
}

fn build_pattern(query: &str) -> Result<Regex, regex::Error> {
    lazy_static! {
        static ref RE: Regex = Regex::new("(.)").unwrap();
    }

    let enhanced = RE.replace_all(&query, ".*($0)");
    Regex::new(&format!("(?i){}", &enhanced))
}

/// Each name is scored and ranked based on similarity to the query.
/// Scores are penalties, 0 is a perfect score and is ranked higher than score 1.
/// A name's score is the number of characters missing in the query for it to be a
/// prefix of the name.
///
/// For example, the score for name "hyperap" given query "hpr" is 2, because two
/// characters are needed to make "hpr" a prefix of "hyperap".
///
/// Characters missing between two characters that both seem to start a "subname" in
/// the name are not counted.
///
/// For example, the score for name "yup-hymn" given query "yhm" is 1; 'y' at index 0
/// starts a subname (by definition), and 'h' starts a subname (because it appears
/// right after a dash), so "up-" is not counted in the score. There is an 'y' between
/// 'h' and 'm' in the name, so it gets a score of 1.
///
/// Note that a query always implicitly matches the subname starting at index 0 in the
/// name. For the query "hyper-mock", the name "yup-hyper-mock" gets a perfect score of
/// 0, because 'y' at index 0 and 'h' at index 4 both start subnames.
///
/// If two names get the same score, the shorter name is ranked first.
pub fn search_names(names: &Vec<String>, input: &str) -> Result<Vec<(String, i32)>, regex::Error> {
    let pattern = build_pattern(input)?;
    let mut results = Vec::new();
    for name in names {
        if let Some(score) = score_name(name, &pattern) {
            results.push((name.clone(), score))
        }
    }

    results.sort_by(|(n1, s1), (n2, s2)| match s1.cmp(&s2) {
        std::cmp::Ordering::Equal => n1.len().cmp(&n2.len()),
        other => other,
    });

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::{build_pattern, score_name};

    #[test]
    fn distance_based() {
        let pattern = build_pattern("hpr").unwrap();
        println!("{:?}", pattern);
        assert_eq!(score_name(&"hyper".to_string(), &pattern), Some(2));
        assert_eq!(score_name(&"hypermegalib".to_string(), &pattern), Some(2));
        assert_eq!(score_name(&"hypexmegalir".to_string(), &pattern), Some(9));
        assert_eq!(score_name(&"yhper".to_string(), &pattern), Some(2));
    }

    #[test]
    fn acronyms() {
        let query = "yhm";
        let pattern = build_pattern(query).unwrap();
        assert_eq!(score_name(&"yup-hyper-mock".to_string(), &pattern), Some(0));
        assert_eq!(score_name(&"yup_hyper_mock".to_string(), &pattern), Some(0));
        assert_eq!(score_name(&"yupHyperMock".to_string(), &pattern), Some(0));
        assert_eq!(score_name(&"yHemr".to_string(), &pattern), Some(1));
        assert_eq!(score_name(&"yup-hymn".to_string(), &pattern), Some(1));
    }
}
