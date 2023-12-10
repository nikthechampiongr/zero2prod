use validator::validate_email;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl SubscriberEmail {
    pub fn parse(email: impl ToString) -> Result<Self, String> {
        let email = email.to_string();
        if !validate_email(&email) {
            return Err("Invalid email".to_string());
        }
        Ok(Self(email))
    }
}

#[cfg(test)]
mod email_tests {
    use super::SubscriberEmail;
    use claims::assert_err;
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;

    #[derive(Clone, Debug)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let email = SafeEmail().fake_with_rng(g);
            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_email(email: ValidEmailFixture) -> bool {
        SubscriberEmail::parse(email.0).is_ok()
    }

    #[test]
    fn empty_email_err() {
        let email = "";

        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn missing_character_err() {
        let email = "ursuladomain.com";

        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn missing_subject_err() {
        let email = "@domain.com";

        assert_err!(SubscriberEmail::parse(email));
    }
}
