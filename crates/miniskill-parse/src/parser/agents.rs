use nom::{
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{char, space0},
    combinator::opt,
    multi::separated_list1,
    sequence::delimited,
    IResult,
};

use crate::types::AgentLink;

// ── Public entry point ────────────────────────────────────────────────────────

/// Parse one line from AGENTS.md. Returns `Some(AgentLink)` if the line is a
/// miniskill link, `None` if it is any other kind of line.
///
/// Expected format:
/// `[miniskill: slug](path) <!-- scope=glob,glob -->`
pub fn parse_agent_link_line(input: &str) -> Option<AgentLink> {
    let input = input.trim();
    parse_agent_link(input).ok().map(|(_, link)| link)
}

fn parse_agent_link(input: &str) -> IResult<&str, AgentLink> {
    // `[miniskill: slug]`
    let (input, _) = char('[')(input)?;
    let (input, _) = tag("miniskill:")(input)?;
    let (input, _) = space0(input)?;
    let (input, slug) = take_until("]")(input)?;
    let (input, _) = char(']')(input)?;

    // `(path)`
    let (input, path) = delimited(char('('), take_until(")"), char(')'))(input)?;

    // Optional `<!-- scope=glob,glob -->`
    let (input, _) = space0(input)?;
    let (input, scope) = opt(parse_scope_comment)(input)?;

    Ok((
        input,
        AgentLink {
            slug: slug.trim().to_owned(),
            path: path.trim().to_owned(),
            scope: scope.unwrap_or_default(),
        },
    ))
}

/// Parse `<!-- scope=glob,glob -->` into a list of glob strings.
fn parse_scope_comment(input: &str) -> IResult<&str, Vec<String>> {
    let (input, _) = tag("<!--")(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("scope=")(input)?;
    let (input, globs) = separated_list1(
        char(','),
        take_while1(|c: char| c != ',' && c != '-' && c != ' ' && c != '>'),
    )(input)?;
    let (input, _) = take_until("-->")(input)?;
    let (input, _) = tag("-->")(input)?;
    Ok((input, globs.into_iter().map(|g| g.trim().to_owned()).collect()))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_link_with_scope() {
        let line = "[miniskill: retro-computing-6502](.miniskills/retro-computing-6502.ms.md) <!-- scope=src/asm/**,*.s,*.asm -->";
        let link = parse_agent_link_line(line).expect("should parse");
        assert_eq!(link.slug, "retro-computing-6502");
        assert_eq!(link.path, ".miniskills/retro-computing-6502.ms.md");
        assert_eq!(link.scope, vec!["src/asm/**", "*.s", "*.asm"]);
    }

    #[test]
    fn link_without_scope() {
        let line = "[miniskill: conlang](.miniskills/conlang.ms.md)";
        let link = parse_agent_link_line(line).expect("should parse");
        assert_eq!(link.slug, "conlang");
        assert_eq!(link.path, ".miniskills/conlang.ms.md");
        assert!(link.scope.is_empty());
    }

    #[test]
    fn non_link_line_returns_none() {
        assert!(parse_agent_link_line("## Miniskills").is_none());
        assert!(parse_agent_link_line("").is_none());
        assert!(parse_agent_link_line("[not a miniskill](somewhere)").is_none());
    }

    #[test]
    fn link_with_description() {
        let line = "[miniskill: cryptography](.miniskills/cryptography.ms.md) <!-- scope=src/crypto/** -->";
        let link = parse_agent_link_line(line).expect("should parse");
        assert_eq!(link.slug, "cryptography");
        assert_eq!(link.scope, vec!["src/crypto/**"]);
    }
}
