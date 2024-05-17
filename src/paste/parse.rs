use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;

use indexmap::IndexMap;
use log::warn;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinePart {
    Text(String),
    Delay(u64),
    Arg(String),
    KeyCombo(Vec<enigo::Key>),
}

pub struct Snippet {
    pub lines: Vec<Vec<LinePart>>,
    pub defaults: IndexMap<String, String>,
}

pub fn snippet_line(
    curr_line: &str,
    lines: &mut Vec<Vec<LinePart>>,
    defaults: &mut IndexMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    enum ProcessingPart {
        Text(String),
        Delay(String),
        Arg {
            value: String,
            default: Option<String>,
        },
        KeyCombo(Vec<String>),
    }
    use ProcessingPart::*;

    let mut result = vec![Text(String::new())];

    let mut chars = curr_line
        .chars()
        .peekable();
    // apparently I cannot use `for ch in chars`,
    // even though the result should be the same. semantics.
    while let Some(ch) = chars.next() {
        match result
            .last_mut()
            .unwrap()
        {
            //
            // Text and staring others
            //

            // start delay with `$'`
            Text(_) if ch == '$' && chars.peek() == Some(&'\'') => {
                chars.next();
                result.push(Delay(String::new()));
            },

            // start arg with `$@`
            Text(_) if ch == '$' && chars.peek() == Some(&'@') => {
                chars.next();
                result.push(Arg {
                    value: String::new(),
                    default: None,
                });
            },

            // start arglist with `$[`
            Text(_) if ch == '$' && chars.peek() == Some(&'[') => {
                todo!("Arglist is not supported yet");
            },

            // start key combo with `$!`
            Text(_) if ch == '$' && chars.peek() == Some(&'!') => {
                chars.next();
                result.push(KeyCombo(vec![String::new()]));
            },

            // just push char at the end of text
            Text(text) => text.push(ch),

            //
            // Delay
            //
            Delay(_) if ch == '$' => {
                result.push(Text(String::new()));
            },

            // just push char at the end of delay string
            Delay(delay) => delay.push(ch),

            //
            // Arg
            //

            // handle `$@$` as `$@` (default is ignored)
            Arg { value, .. } if ch == '$' && value.is_empty() => {
                result.truncate(result.len() - 1);
                match result
                    .last_mut()
                    .unwrap()
                {
                    Text(text) => text.push_str("$@"),
                    _ => result.push(Text("$@".to_string())),
                }
            },

            // handle ending of arg
            Arg { value, default } if ch == '$' => {
                if let Some(default) = default {
                    if let Some(previous) = defaults.insert(
                        value.to_string(),
                        default.to_string(),
                    ) {
                        warn!(
                            "Duplicate default value for argument `{value}`. \
                            Previous value `{previous}` will be ignored."
                        );
                    }
                }

                result.push(Text(String::new()));
            },

            // handle default start
            Arg {
                default: default @ None,
                ..
            } if ch == ':' && chars.peek() == Some(&':') => {
                chars.next();
                *default = Some(String::new());
            },

            // just push char at the end of arg
            Arg {
                value,
                default: None,
            } => value.push(ch),
            Arg {
                default: Some(default),
                ..
            } => default.push(ch),

            //
            // Arglist
            //

            // TODO

            //
            // KeyCombo
            //

            // handle key combo separator
            KeyCombo(combo) if ch == '+' || ch == ' ' => {
                if let Some("") = combo
                    .last()
                    .map(String::as_str)
                {
                    continue;
                }

                combo.push(String::new());
            },

            // handle key combo end
            KeyCombo(..) if ch == '$' => {
                result.push(Text(String::new()));
            },

            // just push char at the end of key combo
            KeyCombo(combo) => combo
                .last_mut()
                .unwrap()
                .push(ch),
        }
    }

    if let Arg { value, .. } = result
        .last()
        .unwrap()
    {
        let escape_result = match value.is_empty() {
            true => r#"text "$@""#.to_string(),
            false => "arg".to_string(),
        };

        warn!(
            // TODO make log crate capture these warnings (so they can be tested for)
            "Argument `$@{value}` is incomplete, \
            you might've wanted to complete it or escape it with `$@$`. \
            Autocompleting as `$@{value}$` ({escape_result})."
        );
    }

    if let KeyCombo(combo) = result
        .last()
        .unwrap()
    {
        let escape_result = match combo.is_empty() {
            true => r#"text "$!""#.to_string(),
            false => "key combo".to_string(),
        };

        let combo = combo
            .iter()
            .map(|x| format!("`{x}`"))
            .collect::<Vec<_>>()
            .join("+");

        warn!(
            "Key combo `$!{combo}` is incomplete, \
            you might've wanted to complete it or escape it with `$!$`. \
            Autocompleting as `$!{combo}$` ({escape_result})."
        );
    }

    // trim trailing empty `Text`
    if let Text(text) = result
        .last()
        .unwrap()
    {
        if text.is_empty() {
            result.pop();
        }
    }

    // save parts as a new line
    let mut line = Vec::new();
    for part in result {
        line.push(match part {
            Text(text) => LinePart::Text(text),
            Delay(delay) => LinePart::Delay(delay.parse()?),
            Arg { value, .. } => LinePart::Arg(value),
            KeyCombo(combo) => {
                let mut res = Vec::new();
                for k in combo {
                    res.push(serde_plain::from_str(&k)?);
                }
                LinePart::KeyCombo(res)
            },
        });
    }
    lines.push(line);
    Ok(())
}

pub fn snippet(snippet: Box<dyn Read>) -> Result<Snippet, Box<dyn std::error::Error>> {
    let mut lines = Vec::new();
    let mut curr_line = String::new();
    let mut appending = false;
    let mut defaults = IndexMap::new();

    for input in BufReader::new(snippet).lines() {
        let input = input?;
        let mut input = input.as_str();

        // trim leading whitespace if appending line to previous one
        if appending {
            appending = false;
            input = input.trim_start();
        }

        // append lines
        // no `input.trim_end().ends_with('\\')`, `\ ` will not append
        // `\\` (double backslash) at end will push first backslash and eat 2nd as an append
        if input.ends_with('\\') {
            curr_line.push_str(&input[..input.len() - 1]);
            appending = true;
            continue;
        }

        // else append to `curr_line`
        curr_line.push_str(input);

        // and `curr_line` is done, parse parts from it
        snippet_line(
            &curr_line,
            &mut lines,
            &mut defaults,
        )?;
        curr_line.clear();
    }

    if appending {
        warn!(
            "Last line ended with an appending backslash `\\`, \
            assuming there is an empty line after it to append nothing. \
            To use a backslash, append it with a whitespace (`\\ `)."
        );
        snippet_line(
            &curr_line,
            &mut lines,
            &mut defaults,
        )?;
    }

    Ok(Snippet { lines, defaults })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snippet_line() {
        let text = "test $@arg1::a$ $@$ $@arg2$";

        let mut lines = Vec::new();
        let mut defaults = IndexMap::new();
        snippet_line(
            text,
            &mut lines,
            &mut defaults,
        )
        .unwrap();

        use LinePart::*;
        assert_eq!(
            lines,
            [[
                Text("test ".to_string()),
                Arg("arg1".to_string()),
                Text(" $@ ".to_string()),
                Arg("arg2".to_string())
            ]]
        );
        assert_eq!(
            defaults,
            IndexMap::from([(
                "arg1".to_string(),
                "a".to_string()
            )])
        );
    }

    #[test]
    fn test_snippet() {
        let text = "\
            test $@arg1$ $@arg2::a$ $@$ $@arg3$\n\
            test $@arg3$ $ $@arg1::b$ $@arg2$";

        let Snippet { lines, defaults } = snippet(Box::new(text.as_bytes())).unwrap();

        use LinePart::*;
        assert_eq!(
            lines,
            [
                &[
                    Text("test ".to_string()),
                    Arg("arg1".to_string()),
                    Text(" ".to_string()),
                    Arg("arg2".to_string()),
                    Text(" $@ ".to_string()),
                    Arg("arg3".to_string()),
                ][..],
                &[
                    Text("test ".to_string()),
                    Arg("arg3".to_string()),
                    Text(" $ ".to_string()),
                    Arg("arg1".to_string()),
                    Text(" ".to_string()),
                    Arg("arg2".to_string()),
                ]
            ]
        );
        assert_eq!(
            defaults,
            IndexMap::from([
                (
                    "arg2".to_string(),
                    "a".to_string()
                ),
                (
                    "arg1".to_string(),
                    "b".to_string()
                )
            ])
        );
    }
}
