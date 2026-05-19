use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{char, space0},
    combinator::map,
    multi::separated_list1,
    sequence::delimited,
    IResult,
};
use std::collections::HashMap;

use crate::types::*;

// ── Public entry point ────────────────────────────────────────────────────────

pub fn parse_token_line(input: &str) -> IResult<&str, MiniskillToken> {
    alt((
        map(parse_self_token, MiniskillToken::Self_),
        map(parse_attest_token, MiniskillToken::Attest),
        map(parse_vouch_token, MiniskillToken::Vouch),
        map(parse_replay_token, MiniskillToken::Replay),
        map(parse_challenge_token, MiniskillToken::Challenge),
        map(parse_response_token, MiniskillToken::Response),
    ))(input)
}

// ── Key-value pair parsing helpers ───────────────────────────────────────────

/// A pipe-delimited sequence of `key=value` pairs, with optional surrounding whitespace.
/// Values may be bare or quoted (`"..."`).
fn parse_kv_pairs(input: &str) -> IResult<&str, HashMap<String, String>> {
    let (input, pairs) = separated_list1(
        delimited(space0, char('|'), space0),
        parse_kv_pair,
    )(input)?;
    Ok((input, pairs.into_iter().collect()))
}

fn parse_kv_pair(input: &str) -> IResult<&str, (String, String)> {
    let (input, key) = take_while1(|c: char| c != '=' && c != '|' && c != '\n')(input)?;
    let (input, _) = char('=')(input)?;
    let (input, value) = alt((parse_quoted_value, parse_bare_value))(input)?;
    Ok((input, (key.trim().to_owned(), value)))
}

fn parse_quoted_value(input: &str) -> IResult<&str, String> {
    let (input, inner) = delimited(char('"'), take_until("\""), char('"'))(input)?;
    Ok((input, inner.to_owned()))
}

fn parse_bare_value(input: &str) -> IResult<&str, String> {
    let (input, v) = take_while1(|c: char| c != '|' && c != '\n')(input)?;
    Ok((input, v.trim().to_owned()))
}

fn kv_get<'a>(map: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    map.get(key).map(|s| s.as_str())
}

fn kv_require<'a>(map: &'a HashMap<String, String>, key: &str) -> Result<&'a str, String> {
    kv_get(map, key).ok_or_else(|| format!("missing required field: {key}"))
}

fn parse_u8_field(map: &HashMap<String, String>, key: &str) -> Result<u8, String> {
    kv_require(map, key)?
        .parse::<u8>()
        .map_err(|e| format!("field `{key}`: {e}"))
}

fn parse_f64_field(map: &HashMap<String, String>, key: &str) -> Option<Result<f64, String>> {
    kv_get(map, key).map(|v| {
        v.parse::<f64>().map_err(|e| format!("field `{key}`: {e}"))
    })
}

fn parse_result_field(map: &HashMap<String, String>) -> Result<AttestResult, String> {
    match kv_require(map, "result")? {
        "PASS" => Ok(AttestResult::Pass),
        "FAIL" => Ok(AttestResult::Fail),
        other => Err(format!("unknown result value: {other}")),
    }
}

// ── MINISKILL-SELF ────────────────────────────────────────────────────────────
//
// Format: MODEL family version | CT-SELF N | topic slug [| version N] [| ai-key key]
// All segments are pipe-delimited with a space-separated `KEYWORD value` structure
// (not `key=value`). Parse by collecting all segments, then interpreting each.

fn parse_self_token(input: &str) -> IResult<&str, SelfToken> {
    let (input, _) = tag("MINISKILL-SELF:")(input)?;
    let (input, _) = space0(input)?;

    // Collect all pipe-delimited segments as raw strings.
    let (input, segments) = separated_list1(
        delimited(space0, char('|'), space0),
        take_while1(|c: char| c != '|' && c != '\n'),
    )(input)?;

    let mut model_family = String::new();
    let mut model_version = String::new();
    let mut ct_self: u8 = 0;
    let mut topic = String::new();
    let mut version = None;
    let mut ai_key = None;

    for seg in segments {
        let seg = seg.trim();
        if let Some(rest) = seg.strip_prefix("MODEL ") {
            // Split on the last space: "claude opus-4-7" → family="claude", version="opus-4-7"
            if let Some((fam, ver)) = rest.rsplit_once(' ') {
                model_family = fam.to_owned();
                model_version = ver.to_owned();
            } else {
                model_family = rest.to_owned();
            }
        } else if let Some(rest) = seg.strip_prefix("CT-SELF ") {
            ct_self = rest.trim().parse().unwrap_or(0);
        } else if let Some(rest) = seg.strip_prefix("topic ") {
            topic = rest.to_owned();
        } else if let Some(rest) = seg.strip_prefix("version ") {
            version = Some(rest.to_owned());
        } else if let Some(rest) = seg.strip_prefix("ai-key ") {
            ai_key = Some(rest.to_owned());
        }
    }

    Ok((input, SelfToken { model_family, model_version, ct_self, topic, version, ai_key }))
}

// ── MINISKILL-ATTEST ──────────────────────────────────────────────────────────

fn parse_attest_token(input: &str) -> IResult<&str, AttestToken> {
    let (input, _) = tag("MINISKILL-ATTEST:")(input)?;
    let (input, _) = space0(input)?;
    let (input, kv) = parse_kv_pairs(input)?;

    let token = (|| -> Result<AttestToken, String> {
        Ok(AttestToken {
            topic: kv_require(&kv, "topic")?.to_owned(),
            version: kv_require(&kv, "version")?.to_owned(),
            model: kv_require(&kv, "model")?.to_owned(),
            ct_required: parse_u8_field(&kv, "ct-required")?,
            ct_claimed: parse_u8_field(&kv, "ct-claimed")?,
            result: parse_result_field(&kv)?,
            proof_system: kv_get(&kv, "proof-system").map(ToOwned::to_owned),
            proof: kv_get(&kv, "proof").map(ToOwned::to_owned),
            vk_hash: kv_get(&kv, "vk-hash").map(ToOwned::to_owned),
            sig: kv_get(&kv, "sig").map(ToOwned::to_owned),
        })
    })();

    token
        .map(|t| (input, t))
        .map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Verify)))
}

// ── MINISKILL-VOUCH ───────────────────────────────────────────────────────────

fn parse_vouch_token(input: &str) -> IResult<&str, VouchToken> {
    let (input, _) = tag("MINISKILL-VOUCH:")(input)?;
    let (input, _) = space0(input)?;
    let (input, kv) = parse_kv_pairs(input)?;

    let token = (|| -> Result<VouchToken, String> {
        Ok(VouchToken {
            topic: kv_require(&kv, "topic")?.to_owned(),
            version: kv_require(&kv, "version")?.to_owned(),
            vouching_for: kv_require(&kv, "vouching-for")?.to_owned(),
            reason: kv_get(&kv, "reason").map(ToOwned::to_owned),
            sig: kv_require(&kv, "sig")?.to_owned(),
        })
    })();

    token
        .map(|t| (input, t))
        .map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Verify)))
}

// ── MINISKILL-REPLAY ──────────────────────────────────────────────────────────

fn parse_replay_token(input: &str) -> IResult<&str, ReplayToken> {
    let (input, _) = tag("MINISKILL-REPLAY:")(input)?;
    let (input, _) = space0(input)?;
    let (input, kv) = parse_kv_pairs(input)?;

    let token = (|| -> Result<ReplayToken, String> {
        Ok(ReplayToken {
            topic: kv_require(&kv, "topic")?.to_owned(),
            ct_required: parse_u8_field(&kv, "ct-required")?,
            ct_claimed: parse_u8_field(&kv, "ct-claimed")?,
            probe_score: parse_f64_field(&kv, "probe-score").transpose()?,
            threshold: parse_f64_field(&kv, "threshold").transpose()?,
            result: parse_result_field(&kv)?,
            enforcer: kv_get(&kv, "enforcer").map(ToOwned::to_owned),
        })
    })();

    token
        .map(|t| (input, t))
        .map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Verify)))
}

// ── MINISKILL-CHALLENGE ───────────────────────────────────────────────────────
//
// Format: topic=slug | challenge-id=id [| source=checker-bot] | "prompt text"
// The prompt is the last field and must be quoted.

fn parse_challenge_token(input: &str) -> IResult<&str, ChallengeToken> {
    let (input, _) = tag("MINISKILL-CHALLENGE:")(input)?;
    let (input, _) = space0(input)?;
    let (input, kv) = parse_kv_pairs(input)?;

    let token = (|| -> Result<ChallengeToken, String> {
        Ok(ChallengeToken {
            topic: kv_require(&kv, "topic")?.to_owned(),
            challenge_id: kv_require(&kv, "challenge-id")?.to_owned(),
            source: kv_get(&kv, "source").map(ToOwned::to_owned),
            prompt: kv_require(&kv, "prompt")
                .map_err(|_| "missing prompt".to_owned())?
                .to_owned(),
        })
    })();

    token
        .map(|t| (input, t))
        .map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Verify)))
}

// ── MINISKILL-RESPONSE ────────────────────────────────────────────────────────

fn parse_response_token(input: &str) -> IResult<&str, ResponseToken> {
    let (input, _) = tag("MINISKILL-RESPONSE:")(input)?;
    let (input, _) = space0(input)?;
    let (input, kv) = parse_kv_pairs(input)?;

    let token = (|| -> Result<ResponseToken, String> {
        Ok(ResponseToken {
            challenge_id: kv_require(&kv, "challenge-id")?.to_owned(),
            respondent: kv_require(&kv, "respondent")?.to_owned(),
            body: kv_require(&kv, "body")
                .or_else(|_| kv_require(&kv, "response"))
                .unwrap_or("")
                .to_owned(),
            sig: kv_get(&kv, "sig").map(ToOwned::to_owned),
        })
    })();

    token
        .map(|t| (input, t))
        .map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Verify)))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emitter::tokens::emit_token;

    fn round_trip(line: &str) {
        let (rest, token) = parse_token_line(line).expect("parse failed");
        assert!(rest.trim().is_empty(), "unconsumed input: {rest:?}");
        let emitted = emit_token(&token);
        let (rest2, token2) = parse_token_line(&emitted).expect("re-parse failed");
        assert!(rest2.trim().is_empty());
        assert_eq!(token, token2, "round-trip mismatch");
    }

    #[test]
    fn self_token_basic() {
        round_trip("MINISKILL-SELF: MODEL claude sonnet-4-6 | CT-SELF 3 | topic retro-computing-6502");
    }

    #[test]
    fn self_token_with_version_and_ai_key() {
        round_trip("MINISKILL-SELF: MODEL claude opus-4-7 | CT-SELF 4 | topic cryptography | version 0.0.1 | ai-key abc123");
    }

    #[test]
    fn attest_token_standard_sig() {
        round_trip(
            "MINISKILL-ATTEST: topic=cryptography | version=0.0.1 | model=claude opus-4-7 | \
             ct-required=4 | ct-claimed=4 | result=PASS | sig=aGVsbG8=",
        );
    }

    #[test]
    fn attest_token_volar() {
        round_trip(
            "MINISKILL-ATTEST: topic=cryptography | version=0.0.1 | model=claude opus-4-7 | \
             ct-required=4 | ct-claimed=4 | result=PASS | proof-system=volar | \
             proof=dGVzdA== | vk-hash=sha256:deadbeef",
        );
    }

    #[test]
    fn vouch_token() {
        round_trip(
            r#"MINISKILL-VOUCH: topic=cryptography | version=0.0.1 | vouching-for=sha256:abcd | reason="reviewed and approved" | sig=c2lnbmF0dXJl"#,
        );
    }

    #[test]
    fn replay_token() {
        round_trip(
            "MINISKILL-REPLAY: topic=cryptography | ct-required=4 | ct-claimed=4 | \
             probe-score=0.82 | threshold=0.75 | result=PASS | enforcer=gate.wasm@0.0.1",
        );
    }

    #[test]
    fn challenge_token() {
        let line = r#"MINISKILL-CHALLENGE: topic=cryptography | challenge-id=c7f3a | source=checker-bot | prompt="What is the SID base address?""#;
        let (rest, token) = parse_token_line(line).expect("parse failed");
        assert!(rest.trim().is_empty());
        if let MiniskillToken::Challenge(c) = token {
            assert_eq!(c.topic, "cryptography");
            assert_eq!(c.challenge_id, "c7f3a");
            assert_eq!(c.source.as_deref(), Some("checker-bot"));
            assert_eq!(c.prompt, "What is the SID base address?");
        } else {
            panic!("wrong token type");
        }
    }

    #[test]
    fn response_token() {
        let line = r#"MINISKILL-RESPONSE: challenge-id=c7f3a | respondent=claude opus-4-7 | body="SID base is $D400""#;
        let (rest, token) = parse_token_line(line).expect("parse failed");
        assert!(rest.trim().is_empty());
        if let MiniskillToken::Response(r) = token {
            assert_eq!(r.challenge_id, "c7f3a");
            assert_eq!(r.body, "SID base is $D400");
        } else {
            panic!("wrong token type");
        }
    }

    #[test]
    fn result_fail() {
        let line = "MINISKILL-ATTEST: topic=foo | version=0.0.1 | model=gpt-5 | \
                    ct-required=3 | ct-claimed=2 | result=FAIL";
        let (_, token) = parse_token_line(line).expect("parse failed");
        if let MiniskillToken::Attest(a) = token {
            assert_eq!(a.result, AttestResult::Fail);
        } else {
            panic!("wrong token type");
        }
    }
}
