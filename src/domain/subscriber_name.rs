use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

static FORBIDDEN_CHARACTERS: [char; 9] = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

fn is_valid_name(s: &str) -> bool {
    let is_empty_or_whitespace = s.trim().is_empty();
    let is_too_long = s.graphemes(true).count() > 256;
    let contains_forbidden_chars = s.chars().any(|g| FORBIDDEN_CHARACTERS.contains(&g));

    !(is_empty_or_whitespace || is_too_long || contains_forbidden_chars)
}

impl SubscriberName {
    pub fn parse(s: String) -> Result<SubscriberName, String> {
        if !is_valid_name(&s) {
            Err(format!("{} is not a valid subscriber name!", s))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::subscriber_name::FORBIDDEN_CHARACTERS;
    use crate::domain::SubscriberName;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "ë".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &FORBIDDEN_CHARACTERS {
            let name = name.to_string();
            assert_err!(SubscriberName::parse(name));
        }
    }

    #[test]
    fn parsing_a_valid_name_successfully() {
        let name = "Gregory Sech".to_string();
        assert_ok!(SubscriberName::parse(name));
    }
}
