// Copyright 2024 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use std::fmt;
use std::io::{self, Read};

#[derive(Debug)]
struct FileDiff<'a> {
    header: &'a str,
    chunks: Vec<Chunk<'a>>,
}

impl<'a> fmt::Display for FileDiff<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.header)?;
        for chunk in &self.chunks {
            write!(f, "{chunk}")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct Chunk<'a> {
    header: &'a str,
    blocks: Vec<ChunkBlock<'a>>,
}

impl<'a> fmt::Display for Chunk<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.header)?;
        for block in &self.blocks {
            write!(f, "{block}")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
enum ChunkBlock<'a> {
    Context(Vec<&'a str>),
    Changed(Changed<'a>),
}

impl<'a> fmt::Display for ChunkBlock<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunkBlock::Context(lines) => {
                for line in lines {
                    writeln!(f, " {line}")?;
                }
            }
            ChunkBlock::Changed(changed) => {
                write!(f, "{changed}")?;
            }
        };
        Ok(())
    }
}

#[derive(Debug)]
struct Changed<'a> {
    removed: Vec<&'a str>,
    added: Vec<&'a str>,
}

impl<'a> fmt::Display for Changed<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in &self.removed {
            writeln!(f, "-{line}")?;
        }
        for line in &self.added {
            writeln!(f, "+{line}")?;
        }
        Ok(())
    }
}

// TODO: Think of an actual abstraction :)
struct Replacement {
    before: &'static str,
    after: &'static str,
}

const REPLACEMENTS: &[Replacement] = &[Replacement {
    before: "NOTREACHED_NORETURN",
    after: "NOTREACHED",
}];

fn parse_file_diffs(input: &str) -> Vec<FileDiff> {
    // diff --git a/ash/accelerators/accelerator_capslock_state_machine.cc b/ash/accelerators/accelerator_capslock_state_machine.cc
    // index 28c373b242560..75f0f75e738a2 100644
    // --- a/ash/accelerators/accelerator_capslock_state_machine.cc
    // +++ b/ash/accelerators/accelerator_capslock_state_machine.cc
    static FILE_HEADER_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(concat!(
            r"(?m)",
            r"^(?:diff --git a/.+ b/.+\nindex [0-9a-f]+..[0-9a-f]+ \d+\n)?",
            r"--- .+\n",
            r"[+]{3} .+\n",
        ))
        .unwrap()
    });
    // @@ -27,8 +27,8 @@ AcceleratorCapslockStateMachine::AcceleratorCapslockStateMachine(
    static CHUNK_HEADER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)@@ .+\n").unwrap());

    let file_headers = FILE_HEADER_RE
        .find_iter(input)
        .map(Some)
        .chain(Some(None))
        .collect::<Vec<_>>();

    file_headers
        .iter()
        .zip(file_headers.iter().skip(1))
        .map(|(current, next)| {
            // By construction, there should always be a `current`.
            let current = current.unwrap();
            let header = current.as_str();

            let file_diff_text = match next {
                Some(next) => &input[current.end()..next.start()],
                None => &input[current.start()..],
            };

            let chunk_headers = CHUNK_HEADER_RE
                .find_iter(file_diff_text)
                .map(Some)
                .chain(Some(None))
                .collect::<Vec<_>>();

            let chunks = chunk_headers
                .iter()
                .zip(chunk_headers.iter().skip(1))
                .map(|(current, next)| {
                    // By construction, there should always be a `current`.
                    let current = current.unwrap();
                    let header = current.as_str();

                    let chunk_text = match next {
                        Some(next) => &file_diff_text[current.end()..next.start()],
                        None => &file_diff_text[current.end()..],
                    };

                    let chunk_text_lines = chunk_text
                        .lines()
                        .map(|line| line.split_at(1))
                        .collect::<Vec<_>>();
                    let blocks = chunk_text_lines
                        .chunk_by(|&(a, _), &(b, _)| a == b || a == "-" && b == "+")
                        .map(|lines| {
                            let (removed, added) = lines.iter().fold(
                                (Vec::new(), Vec::new()),
                                |(mut removed, mut added), &(prefix, line)| {
                                    match prefix {
                                        " " => (),
                                        "-" => removed.push(line),
                                        "+" => added.push(line),
                                        "\\" => assert_eq!(line, " No newline at end of file"),
                                        _ => panic!("unexpected prefix {prefix} at {line}!"),
                                    };
                                    (removed, added)
                                },
                            );
                            if removed.is_empty() && added.is_empty() {
                                ChunkBlock::Context(
                                    lines.iter().map(|(_prefix, line)| line).copied().collect(),
                                )
                            } else {
                                ChunkBlock::Changed(Changed { removed, added })
                            }
                        })
                        .collect::<Vec<_>>();

                    Chunk { header, blocks }
                })
                .collect::<Vec<_>>();

            FileDiff { header, chunks }
        })
        .collect()
}

fn process_file_diffs(file_diffs: Vec<FileDiff>) -> Vec<FileDiff> {
    file_diffs
        .into_iter()
        .filter_map(|FileDiff { header, chunks }| {
            let chunks = chunks
                .into_iter()
                .filter_map(|Chunk { header, blocks }| {
                    let new_blocks = blocks
                        .into_iter()
                        .filter_map(|block| match block {
                            ChunkBlock::Changed(changed) => process_changed_block(changed),
                            ChunkBlock::Context(_) => Some(block),
                        })
                        .collect::<Vec<_>>();
                    // The filtered diff here may not actually apply to the original files. A given
                    // chunk may have multiple changed blocks, but the filtering mechanism used
                    // here does not restore those to "not changed" lines; it just drops them. This
                    // means that there may be context lines that don't correspond to anything. Oh
                    // well :)
                    if new_blocks
                        .iter()
                        .any(|block| matches!(block, ChunkBlock::Changed(_)))
                    {
                        Some(Chunk {
                            header,
                            blocks: new_blocks,
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            if chunks.is_empty() {
                None
            } else {
                Some(FileDiff { header, chunks })
            }
        })
        .collect()
}

fn process_changed_block(changed: Changed) -> Option<ChunkBlock> {
    // TODO: For now, hardcode the checks.
    if changed.removed.is_empty() || changed.added.is_empty() {
        Some(ChunkBlock::Changed(changed))
    } else {
        // Simplifying heuristics:
        // 1. Whitespace is not significant, so join the lines and squash consecutive runs of
        //    whitespace characters into a space.
        // 2. Since the above heuristic tends to produce `( `, e.g. when a function call is
        //    reflowed to the following line, convert `( ` back to `(`.
        // 3. Strip the comment delimiter from lines starting with `//` to improve fuzzy matching
        //    when comments are reflowed across lines.
        // TODO: Perhaps these heuristics should be configurable.
        fn apply_heuristics(lines: &[&str]) -> String {
            static MULTIPLE_WHITESPACE_RE: Lazy<Regex> =
                Lazy::new(|| Regex::new(r"\s{2,}").unwrap());
            fn trim_leading_comment(s: &str) -> &str {
                let s = s.trim_start();
                let s = s.strip_prefix("// ").unwrap_or(s);
                s
            }

            MULTIPLE_WHITESPACE_RE
                .replace_all(
                    &lines
                        .iter()
                        .copied()
                        .map(trim_leading_comment)
                        .collect::<Vec<_>>()
                        .join(" "),
                    " ",
                )
                .into_owned()
                .replace("( ", "(")
        }
        let removed_text = apply_heuristics(&changed.removed);
        let added_text = apply_heuristics(&changed.added);
        // Attempt to transform the before (aka removed) to the after (aka
        // added). Is this efficient? Not particularly. Does it work? Ish.
        let transformed_text = REPLACEMENTS
            .iter()
            .fold(removed_text, |current, replacement| {
                current.replace(replacement.before, replacement.after)
            });
        if transformed_text == added_text {
            // TODO: Maybe this should return ChunkBlock::Elided or something?
            None
        } else {
            Some(ChunkBlock::Changed(changed))
        }
    }
}

fn main() -> Result<()> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let input = input;

    let file_diffs = parse_file_diffs(&input);

    let processed_diffs = process_file_diffs(file_diffs);

    for file in processed_diffs {
        println!("{file}");
    }

    Ok(())
}
