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

// TODO: Redo the representation to simplify the loop. A Vec is not needed here, as a chunk always
// has:
// - before lines
// - the changed lines
// - after lines
#[derive(Debug)]
struct Chunk<'a> {
    header: &'a str,
    lineses: Vec<ChunkLines<'a>>,
}

impl<'a> fmt::Display for Chunk<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.header)?;
        for chunk_lines in &self.lineses {
            write!(f, "{chunk_lines}")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
enum ChunkLines<'a> {
    Context(Vec<&'a str>),
    Changed(Changed<'a>),
}

impl<'a> fmt::Display for ChunkLines<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: Fix the formatting here. There tend to be extra newlines, probably due to
        // unprincipled parsing :)
        match self {
            ChunkLines::Context(lines) => {
                for line in lines {
                    writeln!(f, "{line}")?;
                }
            }
            ChunkLines::Changed(changed) => {
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

const REPLACEMENTS: &[Replacement] = &[
    Replacement {
        before: "ET_UNKNOWN",
        after: "EventType::kUnknown",
    },
    Replacement {
        before: "ET_MOUSE_PRESSED",
        after: "EventType::kMousePressed",
    },
    Replacement {
        before: "ET_MOUSE_DRAGGED",
        after: "EventType::kMouseDragged",
    },
    Replacement {
        before: "ET_MOUSE_RELEASED",
        after: "EventType::kMouseReleased",
    },
    Replacement {
        before: "ET_MOUSE_MOVED",
        after: "EventType::kMouseMoved",
    },
    Replacement {
        before: "ET_MOUSE_ENTERED",
        after: "EventType::kMouseEntered",
    },
    Replacement {
        before: "ET_MOUSE_EXITED",
        after: "EventType::kMouseExited",
    },
    Replacement {
        before: "ET_KEY_PRESSED",
        after: "EventType::kKeyPressed",
    },
    Replacement {
        before: "ET_KEY_RELEASED",
        after: "EventType::kKeyReleased",
    },
    Replacement {
        before: "ET_MOUSEWHEEL",
        after: "EventType::kMousewheel",
    },
    Replacement {
        before: "ET_MOUSE_CAPTURE_CHANGED",
        after: "EventType::kMouseCaptureChanged",
    },
    Replacement {
        before: "ET_TOUCH_RELEASED",
        after: "EventType::kTouchReleased",
    },
    Replacement {
        before: "ET_TOUCH_PRESSED",
        after: "EventType::kTouchPressed",
    },
    Replacement {
        before: "ET_TOUCH_MOVED",
        after: "EventType::kTouchMoved",
    },
    Replacement {
        before: "ET_TOUCH_CANCELLED",
        after: "EventType::kTouchCancelled",
    },
    Replacement {
        before: "ET_DROP_TARGET_EVENT",
        after: "EventType::kDropTargetEvent",
    },
    Replacement {
        before: "ET_GESTURE_SCROLL_BEGIN",
        after: "EventType::kGestureScrollBegin",
    },
    Replacement {
        before: "ET_GESTURE_TYPE_START",
        after: "EventType::kGestureTypeStart",
    },
    Replacement {
        before: "ET_GESTURE_SCROLL_END",
        after: "EventType::kGestureScrollEnd",
    },
    Replacement {
        before: "ET_GESTURE_SCROLL_UPDATE",
        after: "EventType::kGestureScrollUpdate",
    },
    Replacement {
        before: "ET_GESTURE_TAP_DOWN",
        after: "EventType::kGestureTapDown",
    },
    Replacement {
        before: "ET_GESTURE_TAP_CANCEL",
        after: "EventType::kGestureTapCancel",
    },
    Replacement {
        before: "ET_GESTURE_TAP_UNCONFIRMED",
        after: "EventType::kGestureTapUnconfirmed",
    },
    // This is out-of-order from the header files, because this would otherwise disrupt the
    // replacements for ET_GESTURE_TAP_* above. Using a regex engine with word boundaries could
    // prevent this...
    Replacement {
        before: "ET_GESTURE_TAP",
        after: "EventType::kGestureTap",
    },
    Replacement {
        before: "ET_GESTURE_DOUBLE_TAP",
        after: "EventType::kGestureDoubleTap",
    },
    Replacement {
        before: "ET_GESTURE_BEGIN",
        after: "EventType::kGestureBegin",
    },
    Replacement {
        before: "ET_GESTURE_END",
        after: "EventType::kGestureEnd",
    },
    Replacement {
        before: "ET_GESTURE_TWO_FINGER_TAP",
        after: "EventType::kGestureTwoFingerTap",
    },
    Replacement {
        before: "ET_GESTURE_PINCH_BEGIN",
        after: "EventType::kGesturePinchBegin",
    },
    Replacement {
        before: "ET_GESTURE_PINCH_END",
        after: "EventType::kGesturePinchEnd",
    },
    Replacement {
        before: "ET_GESTURE_PINCH_UPDATE",
        after: "EventType::kGesturePinchUpdate",
    },
    Replacement {
        before: "ET_GESTURE_SHORT_PRESS",
        after: "EventType::kGestureShortPress",
    },
    Replacement {
        before: "ET_GESTURE_LONG_PRESS",
        after: "EventType::kGestureLongPress",
    },
    Replacement {
        before: "ET_GESTURE_LONG_TAP",
        after: "EventType::kGestureLongTap",
    },
    Replacement {
        before: "ET_GESTURE_SWIPE",
        after: "EventType::kGestureSwipe",
    },
    Replacement {
        before: "ET_GESTURE_SHOW_PRESS",
        after: "EventType::kGestureShowPress",
    },
    Replacement {
        before: "ET_SCROLL_FLING_START",
        after: "EventType::kScrollFlingStart",
    },
    Replacement {
        before: "ET_SCROLL_FLING_CANCEL",
        after: "EventType::kScrollFlingCancel",
    },
    // This is out-of-order from the header files, because this would otherwise disrupt the
    // replacements for ET_SCROLL_* above. Using a regex engine with word boundaries could prevent
    // this...
    Replacement {
        before: "ET_SCROLL",
        after: "EventType::kScroll",
    },
    Replacement {
        before: "ET_GESTURE_TYPE_END",
        after: "EventType::kGestureTypeEnd",
    },
    Replacement {
        before: "ET_CANCEL_MODE",
        after: "EventType::kCancelMode",
    },
    Replacement {
        before: "ET_UMA_DATA",
        after: "EventType::kUmaData",
    },
    Replacement {
        before: "ET_LAST",
        after: "EventType::kLast",
    },
];

fn main() -> Result<()> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let input = input;

    // diff --git a/ash/accelerators/accelerator_capslock_state_machine.cc b/ash/accelerators/accelerator_capslock_state_machine.cc
    // index 28c373b242560..75f0f75e738a2 100644
    // --- a/ash/accelerators/accelerator_capslock_state_machine.cc
    // +++ b/ash/accelerators/accelerator_capslock_state_machine.cc
    let file_header_re = Regex::new(concat!(
        r"(?m)",
        r"^(?:diff --git a/.+ b/.+\nindex [0-9a-f]+..[0-9a-f]+ \d+\n)?",
        r"--- .+\n",
        r"[+]{3} .+\n",
    ))?;
    // @@ -27,8 +27,8 @@ AcceleratorCapslockStateMachine::AcceleratorCapslockStateMachine(
    let chunk_header_re = Regex::new(r"(?m)@@ .+\n")?;
    let multiple_whitespace_re = Regex::new(r"\s{2,}")?;

    let file_headers = file_header_re
        .find_iter(&input)
        .map(Some)
        .chain(Some(None))
        .collect::<Vec<_>>();

    let files = file_headers
        .iter()
        .zip(file_headers.iter().skip(1))
        .map(|(current, next)| {
            // By construction, there should always be a `current`.
            let current = current.unwrap();
            let header = current.as_str();

            let chunks_text = match next {
                Some(next) => &input[current.end()..next.start()],
                None => &input[current.start()..],
            };

            let chunk_headers = chunk_header_re
                .find_iter(chunks_text)
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
                        Some(next) => &chunks_text[current.end()..next.start()],
                        None => &chunks_text[current.end()..],
                    };

                    let chunk_text_lines = chunk_text
                        .lines()
                        .map(|line| line.split_at(1))
                        .collect::<Vec<_>>();
                    let chunk_lineses = chunk_text_lines
                        .chunk_by(|&(a, _), &(b, _)| a == b || a == "-" && b == "+")
                        .map(
                            |lines| match (lines.first().unwrap().0, lines.last().unwrap().0) {
                                (" ", " ") => ChunkLines::Context(
                                    lines.iter().map(|(_prefix, line)| line).cloned().collect(),
                                ),
                                ("+", "+") | ("-", "+") | ("-", "-") => {
                                    // Assumption: in a given chunk with changed lines, all `_`
                                    // lines and all `+` lines are grouped together.
                                    let removed_lines_count = lines
                                        .iter()
                                        .take_while(|(prefix, _line)| *prefix == "-")
                                        .count();
                                    let (removed, added) = lines.split_at(removed_lines_count);
                                    let removed = removed
                                        .iter()
                                        .map(|(_prefix, line)| line)
                                        .cloned()
                                        .collect();
                                    let added =
                                        added.iter().map(|(_prefix, line)| line).cloned().collect();
                                    ChunkLines::Changed(Changed { removed, added })
                                }
                                chars => panic!("Unexpected prefix characters: {chars:?}"),
                            },
                        )
                        .collect::<Vec<_>>();

                    Chunk {
                        header,
                        lineses: chunk_lineses,
                    }
                })
                .collect::<Vec<_>>();

            FileDiff { header, chunks }
        })
        .collect::<Vec<_>>();

    let files = files
        .into_iter()
        .filter_map(|FileDiff { header, chunks }| {
            let chunks = chunks
                .into_iter()
                .filter_map(|Chunk { header, lineses }| {
                    let new_lineses = lineses
                        .into_iter()
                        .filter_map(|chunk_lines| match chunk_lines {
                            ChunkLines::Changed(changed) => {
                                // TODO: For now, hardcode the checks.
                                if changed.removed.is_empty() || changed.added.is_empty() {
                                    Some(ChunkLines::Changed(changed))
                                } else {
                                    // Simplifying assumption: whitespace is not significant, so
                                    // join the removed lines and the add lines together and squash
                                    // whitespace. As a concession to how clang-format (and humans)
                                    // tend to format and reflow code, change any `( ` back to `(`
                                    // to improve the effectiveness of fuzzy matching.
                                    // TODO: Line merging and whitespace collapsing should possibly be
                                    // a preprocessing step.
                                    let removed_text = multiple_whitespace_re
                                        .replace_all(&changed.removed.join(" "), " ")
                                        .into_owned()
                                        .replace("( ", "(");
                                    let added_text = multiple_whitespace_re
                                        .replace_all(&changed.added.join(" "), " ")
                                        .into_owned()
                                        .replace("( ", "(");
                                    // Attempt to transform the before (aka removed) to the after (aka
                                    // added). Is this efficient? Not particularly. Does it work? Ish.
                                    let transformed_text = REPLACEMENTS.iter().fold(
                                        removed_text,
                                        |current, replacement| {
                                            current.replace(replacement.before, replacement.after)
                                        },
                                    );
                                    if transformed_text == added_text {
                                        None
                                    } else {
                                        Some(ChunkLines::Changed(changed))
                                    }
                                }
                            }
                            chunk_lines => Some(chunk_lines),
                        })
                        .collect::<Vec<_>>();
                    // TODO: This is totally dubious but the representation of Chunk is not quite
                    // correct.
                    if new_lineses.iter().any(|chunk_lines| match chunk_lines {
                        ChunkLines::Changed(_) => true,
                        _ => false,
                    }) {
                        Some(Chunk {
                            header,
                            lineses: new_lineses,
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
        .collect::<Vec<_>>();

    for file in files {
        println!("{file}");
    }

    Ok(())
}
