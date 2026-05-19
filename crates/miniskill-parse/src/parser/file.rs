use nom::{
    bytes::complete::{tag, take_until},
    IResult,
};

use crate::types::{MiniskillFile, MiniskillMeta};

// ── Public entry point ────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum ParseError {
    Nom(String),
    Yaml(serde_yaml::Error),
    MissingGateDelimiter,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Nom(s) => write!(f, "parse error: {s}"),
            ParseError::Yaml(e) => write!(f, "front-matter YAML error: {e}"),
            ParseError::MissingGateDelimiter => write!(f, "missing `---gate---` delimiter"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse a complete `.ms.md` file into its three parts.
pub fn parse_miniskill_file(input: &str) -> Result<MiniskillFile, ParseError> {
    // Extract raw YAML front-matter between the first `---\n` markers.
    let (rest, yaml) = extract_frontmatter(input)
        .map_err(|e| ParseError::Nom(format!("{e}")))?;

    let meta: MiniskillMeta = serde_yaml::from_str(yaml).map_err(ParseError::Yaml)?;

    // Split the body on `---gate---` (may be on its own line or inline).
    // We search for the delimiter with optional surrounding newlines.
    let body = rest.trim_start_matches('\n');
    let (skill, gate) = split_on_gate_delimiter(body)?;

    Ok(MiniskillFile {
        meta,
        skill: skill.trim().to_owned(),
        gate: gate.trim().to_owned(),
    })
}

// ── nom helpers ───────────────────────────────────────────────────────────────

/// Extract the YAML string from `---\n<yaml>\n---\n` and return the remainder.
fn extract_frontmatter(input: &str) -> IResult<&str, &str> {
    // Front-matter opener: `---` followed by a newline.
    let (input, _) = tag("---")(input)?;
    let (input, _) = nom::character::complete::newline(input)?;
    // Everything up to the closing `---`.
    let (input, yaml) = take_until("---")(input)?;
    let (input, _) = tag("---")(input)?;
    Ok((input, yaml.trim_end()))
}

fn split_on_gate_delimiter(body: &str) -> Result<(&str, &str), ParseError> {
    // Search for `---gate---` (accepts surrounding whitespace/newlines).
    let delimiter = "---gate---";
    if let Some(pos) = body.find(delimiter) {
        let skill = &body[..pos];
        let after = &body[pos + delimiter.len()..];
        Ok((skill.trim_end(), after.trim_start_matches('\n')))
    } else {
        Err(ParseError::MissingGateDelimiter)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Status, TopicClass, ZkProofSystem};

    const SAMPLE: &str = r#"---
name: retro-computing-6502
version: 0.0.1
topic-class: frozen-knowledge
ct: 3
it: 2
enforcer: retro-computing-6502.gate.wasm
enforcer-vk: retro-computing-6502.gate.wasm.vk
---

## Skill

This is the skill section.

---gate---

## Gate

This is the gate section.
"#;

    #[test]
    fn parses_meta() {
        let f = parse_miniskill_file(SAMPLE).expect("should parse");
        assert_eq!(f.meta.name, "retro-computing-6502");
        assert_eq!(f.meta.version, "0.0.1");
        assert_eq!(f.meta.topic_class, TopicClass::FrozenKnowledge);
        assert_eq!(f.meta.ct, 3);
        assert_eq!(f.meta.it, 2);
        assert_eq!(f.meta.enforcer.as_deref(), Some("retro-computing-6502.gate.wasm"));
        assert_eq!(f.meta.zk_proof_system, None);
        assert_eq!(f.meta.status, None);
    }

    #[test]
    fn parses_skill_and_gate() {
        let f = parse_miniskill_file(SAMPLE).expect("should parse");
        assert!(f.skill.contains("skill section"));
        assert!(f.gate.contains("gate section"));
        assert!(!f.skill.contains("gate section"));
        assert!(!f.gate.contains("skill section"));
    }

    #[test]
    fn with_zk_proof_system() {
        let src = "---\nname: crypto\nversion: 0.0.1\ntopic-class: high-stakes-precision\nct: 4\nit: 4\nzk-proof-system: volar\n---\n\nskill\n\n---gate---\n\ngate\n";
        let f = parse_miniskill_file(src).expect("should parse");
        assert_eq!(f.meta.zk_proof_system, Some(ZkProofSystem::Volar));
    }

    #[test]
    fn missing_gate_delimiter() {
        let src = "---\nname: x\nversion: 0.0.1\ntopic-class: frozen-knowledge\nct: 1\nit: 1\n---\n\nno gate here\n";
        assert!(matches!(
            parse_miniskill_file(src),
            Err(ParseError::MissingGateDelimiter)
        ));
    }

    #[test]
    fn deprecated_status() {
        let src = "---\nname: old\nversion: 0.0.1\ntopic-class: frozen-knowledge\nct: 1\nit: 1\nstatus: deprecated\n---\n\nskill\n\n---gate---\n\ngate\n";
        let f = parse_miniskill_file(src).expect("should parse");
        assert_eq!(f.meta.status, Some(Status::Deprecated));
    }
}
