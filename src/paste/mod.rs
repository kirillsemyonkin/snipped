mod parse;
mod target;

use std::error::Error;
use std::io;
use std::thread::sleep;
use std::time::Duration;

use enigo::Direction::*;
use enigo::Enigo;
use enigo::InputResult;
use enigo::Key;
use enigo::Keyboard;
use enigo::Settings;
use indexmap::IndexMap;

use self::parse::LinePart;
use crate::ask;

pub fn main(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let mut args = args.into_iter();

    let snippet = target::parse_and_load(&match args.next() {
        Some(x) => x,
        None => ask("Target snippet (supports `github:`, `gist:`, `http(s)://`, file)")?,
    })?;

    let parse::Snippet { lines, defaults } = parse::snippet(snippet)?;

    let mut arg_values = IndexMap::<String, String>::new();
    pull_args_from_argv(args, &mut arg_values);

    for line in &lines {
        while let Some(arg) = missing_line_arg(&line, &arg_values) {
            ask_arg(
                arg,
                &defaults,
                &mut arg_values,
            )?;
        }
    }

    let mut enigo = Enigo::new(&Settings::default())?;
    enigo.key(Key::Alt, Press)?;
    enigo.key(Key::Tab, Press)?;
    enigo.key(Key::Tab, Release)?;
    enigo.key(Key::Alt, Release)?;

    for line in lines {
        sleep(Duration::from_millis(100));

        if let Some(LinePart::Text(first)) = line.get(0) {
            if first.starts_with("##") {
                continue;
            }
        }

        for part in line {
            match &part {
                LinePart::Text(text) => type_text(&mut enigo, text)?,
                LinePart::Arg(arg) => type_text(&mut enigo, &arg_values[arg])?,
                LinePart::KeyCombo(keys) => key_combo(&mut enigo, keys)?,
            }
        }

        enigo.key(Key::Return, Click)?;
    }

    Ok(())
}

fn pull_args_from_argv(
    args: impl IntoIterator<Item = String>,
    arg_values: &mut IndexMap<String, String>,
) {
    for (a, b) in args
        .into_iter()
        .collect::<Vec<_>>()
        .chunks_exact(2)
        .map(|a| (&a[0], &a[1]))
    {
        arg_values.insert(a.to_string(), b.to_string());
    }
}

fn missing_line_arg<'a>(
    line: &'a Vec<LinePart>,
    arg_values: &IndexMap<String, String>,
) -> Option<&'a str> {
    line.iter()
        .find_map(|part| match part {
            LinePart::Arg(arg) if !arg_values.contains_key(arg) => Some(arg.as_str()),
            _ => None,
        })
}

fn ask_arg(
    arg: &str,
    defaults: &IndexMap<String, String>,
    arg_values: &mut IndexMap<String, String>,
) -> io::Result<()> {
    let default = match defaults.get(arg) {
        Some(default) => (
            format!(" (Default: `{default}`)"),
            Some(default),
        ),
        None => (String::new(), None),
    };
    let answer = ask(&format!("{arg}{}", default.0))?;
    let value = match answer {
        s if s.is_empty() => default
            .1
            .cloned()
            .unwrap_or_default(),
        s => s,
    };
    arg_values.insert(arg.to_string(), value);
    Ok(())
}

fn type_text(enigo: &mut Enigo, text: &str) -> InputResult<()> {
    for ch in text.chars() {
        enigo.key(Key::Unicode(ch), Click)?;
        sleep(Duration::from_millis(10));
    }
    Ok(())
}

fn key_combo(enigo: &mut Enigo, keys: &Vec<Key>) -> InputResult<()> {
    let mut to_release = Vec::<Key>::new();

    for key in keys {
        match to_release
            .iter()
            .rposition(|x| x == key)
            .map(|i| to_release.remove(i))
        {
            Some(key) => enigo.key(key, Release)?,
            None => {
                enigo.key(*key, Press)?;
                to_release.push(*key);
            },
        }
    }

    for key in to_release
        .into_iter()
        .rev()
    {
        enigo.key(key, Release)?;
    }

    Ok(())
}
