/// Simple sentence segmentation for streaming TTS
/// This is a simplified version for WebSocket streaming use
pub fn split_into_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current_sentence = String::new();
    let mut quote_depth = 0i32; // Track nested quotes

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        current_sentence.push(ch);

        // Track quote state for various quote characters
        // Opening quotes increase depth, closing quotes decrease
        // U+0022 = ", U+201C = ", U+201D = ", U+00AB = «, U+00BB = »
        match ch {
            '"' => {
                // ASCII double quote toggles (same char for open/close)
                if quote_depth > 0 {
                    quote_depth -= 1;
                } else {
                    quote_depth += 1;
                }
            }
            '\u{201C}' | '\u{00AB}' => quote_depth += 1, // Left curly quote, guillemet
            '\u{201D}' | '\u{00BB}' => quote_depth = (quote_depth - 1).max(0), // Right curly quote, guillemet
            _ => {}
        }

        let in_quotes = quote_depth > 0;

        // Check for sentence endings
        if ch == '.' || ch == '!' || ch == '?' {
            // Look ahead to see if this might be an abbreviation, decimal, or ordinal
            let mut is_sentence_end = true;

            if ch == '.' {
                // Check if this is an ordinal number (e.g., "1.", "2.", "10.")
                // Look back to see if the previous character(s) form a number
                if i > 0 {
                    let prev_char = chars[i - 1];
                    if prev_char.is_ascii_digit() {
                        // Check if followed by whitespace and then more content (not end of text)
                        if i + 1 < chars.len() {
                            let next_char = chars[i + 1];
                            // Ordinal followed by space and content = not a sentence end
                            if next_char.is_whitespace() && i + 2 < chars.len() {
                                is_sentence_end = false;
                            }
                        }
                    }
                }

                // Check lookahead patterns
                if is_sentence_end && i + 1 < chars.len() {
                    let next_char = chars[i + 1];
                    // If followed by whitespace and then a lowercase letter, might not be sentence end
                    if next_char.is_whitespace() && i + 2 < chars.len() {
                        let char_after_space = chars[i + 2];
                        if char_after_space.is_lowercase() {
                            is_sentence_end = false;
                        }
                    }
                    // If followed immediately by a digit, it's probably a decimal
                    else if next_char.is_ascii_digit() {
                        is_sentence_end = false;
                    }
                }
            }

            // If we're inside quotes, don't end the sentence unless the quote closes
            // right after the punctuation AND there's no continuation after the quote
            if in_quotes {
                // Check if the next character closes the quote
                if i + 1 < chars.len() {
                    let next_char = chars[i + 1];
                    if next_char == '"' || next_char == '\u{201D}' || next_char == '\u{00BB}' {
                        // Quote closes right after punctuation
                        // Check if there's more content after the closing quote
                        if i + 2 < chars.len() {
                            let char_after_quote = chars[i + 2];
                            if char_after_quote.is_whitespace() && i + 3 < chars.len() {
                                // Check what comes after the space
                                let char_after_space = chars[i + 3];
                                if char_after_space.is_lowercase() {
                                    // Continuation like `"Hello." and then` - NOT a sentence end
                                    is_sentence_end = false;
                                } else {
                                    // New sentence starts with uppercase - IS a sentence end
                                    // Include the closing quote in this sentence
                                    i += 1;
                                    current_sentence.push(chars[i]);
                                    quote_depth = (quote_depth - 1).max(0);
                                }
                            } else if !char_after_quote.is_whitespace() {
                                // No space after quote, continuation - NOT a sentence end
                                is_sentence_end = false;
                            } else {
                                // Just whitespace to end - IS a sentence end
                                i += 1;
                                current_sentence.push(chars[i]);
                                quote_depth = (quote_depth - 1).max(0);
                            }
                        } else {
                            // End of text after quote - IS a sentence end
                            i += 1;
                            current_sentence.push(chars[i]);
                            quote_depth = (quote_depth - 1).max(0);
                        }
                    } else {
                        // Still inside quotes, not a sentence end
                        is_sentence_end = false;
                    }
                } else {
                    // End of text while in quotes - still end the sentence
                    is_sentence_end = true;
                }
            }

            if is_sentence_end {
                // Consume any trailing whitespace
                while i + 1 < chars.len() && chars[i + 1].is_whitespace() {
                    i += 1;
                    current_sentence.push(chars[i]);
                }

                let trimmed = current_sentence.trim().to_string();
                if !trimmed.is_empty() {
                    sentences.push(trimmed);
                }
                current_sentence.clear();
            }
        }

        i += 1;
    }

    // Add any remaining text as the last sentence
    let trimmed = current_sentence.trim().to_string();
    if !trimmed.is_empty() {
        sentences.push(trimmed);
    }

    sentences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_sentences() {
        let result = split_into_sentences("Hello world. How are you?");
        assert_eq!(result, vec!["Hello world.", "How are you?"]);
    }

    #[test]
    fn test_ordinal_numbers() {
        // Ordered lists should NOT be split
        let result = split_into_sentences("1. First item 2. Second item 3. Third item");
        assert_eq!(result, vec!["1. First item 2. Second item 3. Third item"]);

        let result = split_into_sentences("There are 3 steps: 1. Do this 2. Do that 3. Done!");
        assert_eq!(
            result,
            vec!["There are 3 steps: 1. Do this 2. Do that 3. Done!"]
        );
    }

    #[test]
    fn test_decimal_numbers() {
        let result = split_into_sentences("The value is 3.14 and it works.");
        assert_eq!(result, vec!["The value is 3.14 and it works."]);
    }

    #[test]
    fn test_quoted_passages() {
        // Quotes with periods inside should not split prematurely
        let result = split_into_sentences("He said \"Hello. How are you?\" and left.");
        assert_eq!(result, vec!["He said \"Hello. How are you?\" and left."]);

        // Curly quotes - use raw string to include special quote chars
        let result = split_into_sentences("She replied \u{201C}I'm fine. Thanks!\u{201D} quickly.");
        assert_eq!(
            result,
            vec!["She replied \u{201C}I'm fine. Thanks!\u{201D} quickly."]
        );
    }

    #[test]
    fn test_sentence_ending_with_quote() {
        // Quote at end of sentence should end properly
        let result = split_into_sentences("He said \"Hello.\" Then he left.");
        assert_eq!(result, vec!["He said \"Hello.\"", "Then he left."]);
    }

    #[test]
    fn test_abbreviations_lowercase() {
        // Period followed by lowercase = not a sentence end
        let result = split_into_sentences("Dr. smith is here.");
        assert_eq!(result, vec!["Dr. smith is here."]);
    }
}
