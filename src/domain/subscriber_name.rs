use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl SubscriberName {
    pub fn parse(name: impl ToString) -> Result<Self, String> {
        let name = name.to_string();
        let empty = name.trim().is_empty();

        let too_long = name.graphemes(true).count() > 256;

        const FORBIDDEN_CHARS: [char; 9] = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

        let illegal_chars = name.chars().any(|c| FORBIDDEN_CHARS.contains(&c));

        if empty || too_long || illegal_chars {
            Err("Invalid input".to_string())
        } else {
            Ok(Self(name))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberName;
    use claims::{assert_err, assert_ok};

    #[test]
    fn name_valid() {
        let name = "Ursula Le Guin";
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_256_grapheme_long_name_valid() {
        let name = "ё".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_name_longer_than_256_graphemes_err() {
        let name = "ё".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn empty_name_err() {
        let name = "";
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn invalid_caharacter_name_err() {
        let name = "Nik'); DROP TABLE subscriptions --";
        assert_err!(SubscriberName::parse(name));
    }
}
