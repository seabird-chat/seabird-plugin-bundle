use failure::Error;
use serde::Deserialize;

use crate::irc::Message;

#[derive(Debug, PartialEq, Deserialize)]
struct Atoms {
    // TODO: tags
    source: Option<String>,
    verb: String,
    #[serde(default)]
    params: Vec<String>,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Test {
    input: String,
    atoms: Atoms,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Tests {
    tests: Vec<Test>,
}

#[test]
fn test_from_str() -> Result<(), Error> {
    let tests = include_str!("test_data.yml");
    let tests: Tests = serde_yaml::from_str(&tests)?;
    for test in tests.tests {
        let msg = test.input.parse::<Message>()?;
        assert_eq!(msg.command, test.atoms.verb);
        assert_eq!(msg.prefix, test.atoms.source);
        assert_eq!(msg.params, test.atoms.params);
    }

    Ok(())
}
