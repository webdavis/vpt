use super::*;

pub(super) fn tags(source: &str, prefix: Range<usize>) -> Vec<Range<usize>> {
    let mut chars = source[prefix.clone()].char_indices().peekable();
    let mut ranges = Vec::new();
    while let Some((start, ch)) = chars.next() {
        match ch {
            '#' => {
                for (_, ch) in chars.by_ref() {
                    if matches!(ch, '\r' | '\n') {
                        break;
                    }
                }
            }
            '&' => {
                while chars.peek().is_some_and(|(_, ch)| !separator(*ch)) {
                    chars.next();
                }
            }
            '!' => {
                let mut end = start + 1;
                if chars.peek().is_some_and(|(_, ch)| *ch == '<') {
                    for (index, ch) in chars.by_ref() {
                        end = index + ch.len_utf8();
                        if ch == '>' {
                            break;
                        }
                    }
                } else {
                    while let Some(&(index, ch)) = chars.peek() {
                        if separator(ch) {
                            break;
                        }
                        end = index + ch.len_utf8();
                        chars.next();
                    }
                }
                ranges.push(prefix.start + start..prefix.start + end);
            }
            _ => {}
        }
    }
    ranges
}

fn separator(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, ',' | '[' | ']' | '{' | '}')
}
