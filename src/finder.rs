use crate::display;
use crate::structures::cheat::VariableMap;
use crate::structures::finder::{Opts, SuggestionType};
use anyhow::Context;
use anyhow::Error;
use skim::prelude::*;
use std::io::{stdin, Cursor, IoSlice, Write};
use std::process;
use std::process::{Command, Stdio};

impl<'a> From<&'a Opts> for SkimOptions<'a> {
    fn from(opts: &'a Opts) -> SkimOptions<'a> {
        let mut options = SkimOptions::default();
        options.preview_window = Some("up:2");
        options.with_nth = Some("1,2,3");
        options.delimiter = Some(display::DELIMITER);
        options.ansi = true;
        options.bind = vec!["ctrl-j:down", "ctrl-k:up"];
        options.exact = true;

        if opts.autoselect {
            // TODO not implmentend in skim yet
            // options.select_1 = True
        }

        match opts.suggestion_type {
            SuggestionType::MultipleSelections => {
                options.multi = true;
            }
            SuggestionType::Disabled => {
                options.print_query = true;
                options.height = Some("1");
            }
            SuggestionType::SnippetSelection => {
                options.expect = Some("ctrl-y,enter".into());
            }
            SuggestionType::SingleRecommendation => {
                options.print_query = true;
                options.expect = Some("tab,enter".into());
            }
            _ => {}
        }

        if let Some(preview) = &opts.preview {
            options.preview = Some(preview);
        }
        if let Some(query) = &opts.query {
            options.query = Some(query);
        }
        if let Some(filter) = &opts.filter {
            options.filter = filter;
        }
        if let Some(header) = &opts.header {
            options.header = Some(header);
        }
        if let Some(prompt) = &opts.prompt {
            options.prompt = Some(prompt);
        }
        if let Some(preview_window) = &opts.preview_window {
            options.preview_window = Some(preview_window);
        }
        options.header_lines = opts.header_lines.into();

        // TODO overrides
        // options.override = opts.overrides

        options
    }
}

pub fn call<F>(opts: Opts, write_fn: F) -> Result<(String, Option<VariableMap>), Error>
where
    F: Fn(&mut Cursor<Vec<u8>>) -> Result<Option<VariableMap>, Error>,
{
    let options: SkimOptions = SkimOptions::from(&opts);

    let reader = SkimItemReader::default();
    let mut buff = vec![];
    let mut cursor = Cursor::new(buff);

    let map = write_fn(&mut cursor)?;

    let items = reader.of_bufread(cursor);

    Ok(("".into(), None))
}

/*
fn get_column(text: String, column: Option<u8>, delimiter: Option<&str>) -> String {
    if let Some(c) = column {
        let mut result = String::from("");
        let re = regex::Regex::new(delimiter.unwrap_or(r"\s\s+")).expect("Invalid regex");
        for line in text.split('\n') {
            if (&line).is_empty() {
                continue;
            }
            let mut parts = re.split(line).skip((c - 1) as usize);
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(parts.next().unwrap_or(""));
        }
        result
    } else {
        text
    }
}

pub fn call<F>(opts: Opts, stdin_fn: F) -> Result<(String, Option<VariableMap>), Error>
where
    F: Fn(&mut process::ChildStdin) -> Result<Option<VariableMap>, Error>,
{
    let mut fzf_command = Command::new("fzf");

    fzf_command.args(&[
        "--preview-window",
        "up:2",
        "--with-nth",
        "1,2,3",
        "--delimiter",
        display::DELIMITER.to_string().as_str(),
        "--ansi",
        "--bind",
        "ctrl-j:down,ctrl-k:up",
        "--exact",
    ]);

    if opts.autoselect {
        fzf_command.arg("--select-1");
    }

    match opts.suggestion_type {
        SuggestionType::MultipleSelections => {
            fzf_command.arg("--multi");
        }
        SuggestionType::Disabled => {
            fzf_command.args(&["--print-query", "--no-select-1", "--height", "1"]);
        }
        SuggestionType::SnippetSelection => {
            fzf_command.args(&["--expect", "ctrl-y,enter"]);
        }
        SuggestionType::SingleRecommendation => {
            fzf_command.args(&["--print-query", "--expect", "tab,enter"]);
        }
        _ => {}
    }

    if let Some(p) = opts.preview {
        fzf_command.args(&["--preview", &p]);
    }

    if let Some(q) = opts.query {
        fzf_command.args(&["--query", &q]);
    }

    if let Some(f) = opts.filter {
        fzf_command.args(&["--filter", &f]);
    }

    if let Some(h) = opts.header {
        fzf_command.args(&["--header", &h]);
    }

    if let Some(p) = opts.prompt {
        fzf_command.args(&["--prompt", &p]);
    }

    if let Some(pw) = opts.preview_window {
        fzf_command.args(&["--preview-window", &pw]);
    }

    if opts.header_lines > 0 {
        fzf_command.args(&["--header-lines", format!("{}", opts.header_lines).as_str()]);
    }

    if let Some(o) = opts.overrides {
        o.as_str()
            .split(' ')
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .for_each(|s| {
                fzf_command.arg(s);
            });
    }

    let child = fzf_command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn();

    let mut child = match child {
        Ok(x) => x,
        Err(_) => {
            eprintln!("navi was unable to call fzf.\nPlease make sure it's correctly installed\nRefer to https://github.com/junegunn/fzf for more info.");
            process::exit(33)
        }
    };

    let stdin = child
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow!("Unable to acquire stdin of fzf"))?;
    let result_map = stdin_fn(stdin).context("Failed to pass data to fzf")?;

    let out = child.wait_with_output().context("Failed to wait for fzf")?;

    let text = match out.status.code() {
        Some(0) | Some(1) | Some(2) => {
            String::from_utf8(out.stdout).context("Invalid utf8 received from fzf")?
        }
        Some(130) => process::exit(130),
        _ => {
            let err = String::from_utf8(out.stderr)
                .unwrap_or_else(|_| "<stderr contains invalid UTF-8>".to_owned());
            panic!("External command failed:\n {}", err)
        }
    };

    let out = get_column(
        parse_output_single(text, opts.suggestion_type)?,
        opts.column,
        opts.delimiter.as_deref(),
    );

    Ok((out, result_map))
}

fn parse_output_single(mut text: String, suggestion_type: SuggestionType) -> Result<String, Error> {
    Ok(match suggestion_type {
        SuggestionType::SingleSelection => text
            .lines()
            .next()
            .context("Not sufficient data for single selection")?
            .to_string(),
        SuggestionType::MultipleSelections
        | SuggestionType::Disabled
        | SuggestionType::SnippetSelection => {
            let len = text.len();
            if len > 1 {
                text.truncate(len - 1);
            }
            text
        }
        SuggestionType::SingleRecommendation => {
            let lines: Vec<&str> = text.lines().collect();

            match (lines.get(0), lines.get(1), lines.get(2)) {
                (Some(one), Some(termination), Some(two))
                    if *termination == "enter" || termination.is_empty() =>
                {
                    if two.is_empty() {
                        (*one).to_string()
                    } else {
                        (*two).to_string()
                    }
                }
                (Some(one), Some(termination), None)
                    if *termination == "enter" || termination.is_empty() =>
                {
                    (*one).to_string()
                }
                (Some(one), Some(termination), _) if *termination == "tab" => (*one).to_string(),
                _ => "".to_string(),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_output1() {
        let text = "palo\n".to_string();
        let output = parse_output_single(text, SuggestionType::SingleSelection).unwrap();
        assert_eq!(output, "palo");
    }

    #[test]
    fn test_parse_output2() {
        let text = "\nenter\npalo".to_string();
        let output = parse_output_single(text, SuggestionType::SingleRecommendation).unwrap();
        assert_eq!(output, "palo");
    }

    #[test]
    fn test_parse_recommendation_output_1() {
        let text = "\nenter\npalo".to_string();
        let output = parse_output_single(text, SuggestionType::SingleRecommendation).unwrap();
        assert_eq!(output, "palo");
    }

    #[test]
    fn test_parse_recommendation_output_2() {
        let text = "p\nenter\npalo".to_string();
        let output = parse_output_single(text, SuggestionType::SingleRecommendation).unwrap();
        assert_eq!(output, "palo");
    }

    #[test]
    fn test_parse_recommendation_output_3() {
        let text = "peter\nenter\n".to_string();
        let output = parse_output_single(text, SuggestionType::SingleRecommendation).unwrap();
        assert_eq!(output, "peter");
    }

    #[test]
    fn test_parse_output3() {
        let text = "p\ntab\npalo".to_string();
        let output = parse_output_single(text, SuggestionType::SingleRecommendation).unwrap();
        assert_eq!(output, "p");
    }

    #[test]
    fn test_parse_snippet_request() {
        let text = "enter\nssh                     ⠀login to a server and forward to ssh key (d…  ⠀ssh -A <user>@<server>  ⠀ssh  ⠀login to a server and forward to ssh key (dangerous but usefull for bastion hosts)  ⠀ssh -A <user>@<server>  ⠀\n".to_string();
        let output = parse_output_single(text, SuggestionType::SnippetSelection).unwrap();
        assert_eq!(output,     "enter\nssh                     ⠀login to a server and forward to ssh key (d…  ⠀ssh -A <user>@<server>  ⠀ssh  ⠀login to a server and forward to ssh key (dangerous but usefull for bastion hosts)  ⠀ssh -A <user>@<server>  ⠀");
    }
}
*/
