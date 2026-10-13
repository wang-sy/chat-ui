/// Extract the readable response even while the enclosing JSON/HTML is still
/// streaming. Never send the orchestration envelope to the conversation UI.
pub(crate) fn assistant_text(text: &str) -> String {
    let trimmed = text.trim_start();
    let candidate = trimmed
        .strip_prefix("```json")
        .unwrap_or(trimmed)
        .trim_start();
    if !candidate.starts_with('{') {
        return text.to_string();
    }
    let mut depth = 0;
    let mut chars = candidate.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        match ch {
            '{' | '[' => depth += 1,
            '}' | ']' => depth -= 1,
            '"' => {
                let (value, consumed, closed) = string_prefix(&candidate[index..]);
                let end = index + consumed;
                while chars.peek().is_some_and(|(offset, _)| *offset < end) {
                    chars.next();
                }
                if !closed {
                    break;
                }
                if depth == 1 && value == "response" {
                    let rest = candidate[end..].trim_start();
                    if let Some(rest) = rest.strip_prefix(':') {
                        return string_prefix(rest.trim_start()).0;
                    }
                }
            }
            _ => {}
        }
    }
    String::new()
}

fn string_prefix(source: &str) -> (String, usize, bool) {
    if !source.starts_with('"') {
        return (String::new(), 0, false);
    }
    let mut chars = source.char_indices().skip(1).peekable();
    let mut decoded = String::new();
    while let Some((offset, ch)) = chars.next() {
        match ch {
            '"' => return (decoded, offset + 1, true),
            '\\' => {
                let Some((_, escape)) = chars.next() else {
                    break;
                };
                match escape {
                    'n' => decoded.push('\n'),
                    'r' => decoded.push('\r'),
                    't' => decoded.push('\t'),
                    'b' => decoded.push('\u{8}'),
                    'f' => decoded.push('\u{c}'),
                    '"' | '\\' | '/' => decoded.push(escape),
                    'u' => {
                        let mut code = String::new();
                        for _ in 0..4 {
                            if let Some((_, ch)) = chars.next() {
                                code.push(ch);
                            }
                        }
                        let Ok(high) = u16::from_str_radix(&code, 16) else {
                            break;
                        };
                        if code.len() != 4 {
                            break;
                        }
                        let value = if (0xd800..=0xdbff).contains(&high) {
                            if chars.next().map(|(_, c)| c) != Some('\\')
                                || chars.next().map(|(_, c)| c) != Some('u')
                            {
                                break;
                            }
                            let low: String = chars.by_ref().take(4).map(|(_, c)| c).collect();
                            let Ok(low) = u16::from_str_radix(&low, 16) else {
                                break;
                            };
                            if !(0xdc00..=0xdfff).contains(&low) {
                                break;
                            }
                            0x10000 + (((high as u32) - 0xd800) << 10) + (low as u32 - 0xdc00)
                        } else {
                            high as u32
                        };
                        if let Some(ch) = char::from_u32(value) {
                            decoded.push(ch);
                        }
                    }
                    _ => break,
                }
            }
            _ => decoded.push(ch),
        }
    }
    (decoded, source.len(), false)
}

pub(crate) fn user_text(text: &str) -> String {
    if text.starts_with("Runtime integration pass:") {
        return String::new();
    }
    if let Some(value) = text.strip_prefix("User request:") {
        return value
            .split("\n\nCurrent Space snapshot")
            .next()
            .unwrap_or(value)
            .trim()
            .into();
    }
    for marker in ["\n\nAssigned contract:\n", "\n\nUpdated contract:\n"] {
        if (text.starts_with("Create or refine the visual artifact")
            || text.starts_with("The Main Thread assigned"))
            && text.contains(marker)
        {
            let brief = text
                .split_once(marker)
                .map(|(_, brief)| brief)
                .unwrap_or("");
            return format!(
                "Main Thread 分派任务\n\n{}",
                brief
                    .split("\n\nReturn the structured")
                    .next()
                    .unwrap_or(brief)
                    .split("\n\nUpdate only this View")
                    .next()
                    .unwrap_or(brief)
                    .trim()
            );
        }
    }
    text.into()
}

pub(super) fn bounded_output(text: &str) -> String {
    const LIMIT: usize = 16_000;
    if text.chars().count() <= LIMIT {
        return text.into();
    }
    let tail: String = text
        .chars()
        .rev()
        .take(LIMIT)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("…（仅显示最后 16,000 个字符）\n{tail}")
}
