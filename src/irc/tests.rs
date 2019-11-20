use failure::Error;
use std::collections::BTreeMap;

use serde::Deserialize;

use crate::irc::Message;

#[derive(Debug, PartialEq, Deserialize)]
struct Atoms {
    #[serde(default)]
    tags: BTreeMap<String, String>,
    source: Option<String>,
    verb: String,
    #[serde(default)]
    params: Vec<String>,

}

#[derive(Debug, PartialEq, Deserialize)]
struct FromStrTest {
    input: String,
    atoms: Atoms,
}

#[derive(Debug, PartialEq, Deserialize)]
struct FromStrTests {
    tests: Vec<FromStrTest>,
}

#[derive(Debug, PartialEq, Deserialize)]
struct ToStrTest {
    atoms: Atoms,
    #[serde(default)]
    matches: Vec<String>,
}

#[derive(Debug, PartialEq, Deserialize)]
struct ToStrTests {
    tests: Vec<ToStrTest>,
}

#[test]
fn test_from_str() -> Result<(), Error> {
    let tests = include_str!("parser_tests/tests/msg-split.yaml");
    let tests: FromStrTests = serde_yaml::from_str(&tests)?;
    for test in tests.tests {
        let msg = test.input.parse::<Message>()?;
        assert_eq!(msg.tags, test.atoms.tags);
        assert_eq!(msg.command, test.atoms.verb);
        assert_eq!(msg.prefix, test.atoms.source);
        assert_eq!(msg.params, test.atoms.params);
    }

    Ok(())
}

#[test]
fn test_to_string() -> Result<(), Error> {
    let tests = include_str!("parser_tests/tests/msg-join.yaml");
    let tests: ToStrTests = serde_yaml::from_str(&tests)?;
    for test in tests.tests {
        let msg = Message {
            tags: test.atoms.tags,
            prefix: test.atoms.source,
            command: test.atoms.verb,
            params: test.atoms.params,
        };

        println!("{}", msg.to_string());
        assert!(test.matches.contains(&msg.to_string()));
    }

    Ok(())
}
