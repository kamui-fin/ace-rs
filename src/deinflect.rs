use serde_derive::Deserialize;
use std::collections::HashMap;

pub struct Deinflector {
    normalized_reasons: NormalizedReasons,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ReasonInfo {
    kana_in: String,
    kana_out: String,
    rules_in: Vec<String>,
    rules_out: Vec<String>,
}

type Reasons = HashMap<String, Vec<ReasonInfo>>;

#[derive(Debug)]
pub struct NormalizedReasonInfo {
    kana_in: String,
    kana_out: String,
    bits_in: u8,
    bits_out: u8,
}

pub type NormalizedReasons = HashMap<String, Vec<NormalizedReasonInfo>>;

#[derive(Debug, Clone)]
pub struct DeinflectResult {
    pub term: String,
    rules: u8,
    reasons: Vec<String>,
}

impl Deinflector {
    pub fn new(deinflect_json: &'static str) -> Self {
        let mut rule_types: HashMap<&str, u8> = HashMap::new();
        rule_types.insert("v1", 0b00000001); // Verb ichidan
        rule_types.insert("v5", 0b00000010); // Verb godan
        rule_types.insert("vs", 0b00000100); // Verb suru
        rule_types.insert("vk", 0b00001000); // Verb kuru
        rule_types.insert("vz", 0b00010000); // Verb zuru
        rule_types.insert("adj-i", 0b00100000); // Adjective i
        rule_types.insert("iru", 0b01000000); // Intermediate -iru endings for progressive or perfect tense

        let reasons: Reasons = serde_json::from_str::<Reasons>(deinflect_json).unwrap();
        let normalized_reasons: NormalizedReasons = Self::normalize_reasons(reasons, &rule_types);
        Self { normalized_reasons }
    }

    pub fn normalize_reasons(
        reasons: Reasons,
        rule_types: &HashMap<&'static str, u8>,
    ) -> NormalizedReasons {
        let mut normalized_reason: NormalizedReasons = HashMap::new();

        for (reason, reason_info) in reasons.iter() {
            let mut variants: Vec<NormalizedReasonInfo> = vec![];
            for ReasonInfo {
                kana_in,
                kana_out,
                rules_in,
                rules_out,
            } in reason_info
            {
                let bits_in = Self::rule_to_rule_flags(rules_in.to_vec(), &rule_types);
                let bits_out = Self::rule_to_rule_flags(rules_out.to_vec(), &rule_types);

                variants.push(NormalizedReasonInfo {
                    kana_in: kana_in.to_string(),
                    kana_out: kana_out.to_string(),
                    bits_in,
                    bits_out,
                })
            }
            normalized_reason.insert(reason.to_string(), variants);
        }

        normalized_reason
    }

    pub fn deinflect(&self, word: String) -> Vec<DeinflectResult> {
        let mut results: Vec<DeinflectResult> = vec![DeinflectResult {
            term: word,
            rules: 0,
            reasons: vec![],
        }];
        for i in 0..results.len() {
            let curr = results[i].to_owned();
            for (reason, variants) in &self.normalized_reasons {
                for NormalizedReasonInfo {
                    kana_in,
                    kana_out,
                    bits_in,
                    bits_out,
                } in variants
                {
                    if (curr.rules != 0 && (curr.rules & *bits_in) == 0)
                        || !curr.term.ends_with(kana_in)
                        || (curr.term.len() - kana_in.len() + kana_out.len()) <= 0
                    {
                        continue;
                    }

                    let mut rsns = vec![reason.clone()];
                    rsns.extend_from_slice(curr.reasons.as_slice());
                    let new_res = DeinflectResult {
                        term: curr.term[0..curr.term.len() - kana_in.len()].to_string() + &kana_out,
                        rules: *bits_out,
                        reasons: rsns,
                    };
                    results.push(new_res);
                }
            }
        }

        results
    }

    pub fn rule_to_rule_flags(rules: Vec<String>, rule_types: &HashMap<&'static str, u8>) -> u8 {
        let mut value = 0;
        for rule in rules {
            let rule_bits = rule_types.get(rule.as_str());
            if let Some(rule_bits) = rule_bits {
                value |= rule_bits;
            }
        }
        value
    }
}
