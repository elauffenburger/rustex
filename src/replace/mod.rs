use tracing::debug;

use crate::executor;

#[derive(Debug)]
pub struct ReplaceSpec {
    parts: Vec<ReplaceSpecNodeValue>,
}

#[derive(Debug)]
pub enum ReplaceSpecNodeValue {
    String(String),
    GroupNum(String),
}

impl<'a> From<&'a str> for ReplaceSpec {
    fn from(str: &'a str) -> Self {
        ReplaceSpec::parse_str(str)
    }
}

impl ReplaceSpec {
    pub fn parse_str(str: &str) -> Self {
        let mut spec = ReplaceSpec { parts: vec![] };
        let mut chars = str.chars().peekable();

        let mut curr_word = None;
        while let Some(ch) = chars.next() {
            // If this isn't the start of a token replacement, just push it to the in-progress word and continue.
            if ch != '$' {
                match &mut curr_word {
                    None => curr_word = Some(String::from(ch)),
                    Some(word) => word.push(ch),
                }

                continue;
            }

            // Looks like we hit the start of a token replacement; finish up the current word and read the token.
            if let Some(word) = std::mem::take(&mut curr_word) {
                spec.parts.push(ReplaceSpecNodeValue::String(word));
            }

            let group_num = {
                let mut group_num = String::new();
                while let Some(next) = chars.peek() {
                    match *next {
                        '1'..='9' => {
                            group_num.push(*next);
                            chars.next();
                        }
                        _ => break,
                    }
                }

                group_num
            };
            spec.parts.push(ReplaceSpecNodeValue::GroupNum(group_num));
        }

        if let Some(word) = curr_word {
            spec.parts.push(ReplaceSpecNodeValue::String(word));
        }

        spec
    }

    #[tracing::instrument]
    pub fn perform_replace(&self, input: &str, res: &executor::ExecResult) -> Option<String> {
        debug!("performing replace");

        if self.parts.is_empty() {
            return None;
        }

        let replaced = self.parts.iter().fold(String::new(), |mut acc, part| {
            match part {
                ReplaceSpecNodeValue::String(str) => acc.push_str(str),
                ReplaceSpecNodeValue::GroupNum(group_name) => match res.groups.get(group_name) {
                    None => acc.push_str(&format!("${}", group_name)),
                    Some(val) => acc.push_str(&input[val.0..=val.1]),
                },
            }

            acc
        });

        Some(replaced)
    }
}
