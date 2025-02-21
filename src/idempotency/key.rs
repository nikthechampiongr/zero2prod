#[derive(Debug)]
pub struct IdempotencyKey(String);

impl TryFrom<String> for IdempotencyKey {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            anyhow::bail!("The idempotency key cannot be empty");
        }

        const MAX_LENGTH: usize = 50;

        if value.len() >= MAX_LENGTH {
            anyhow::bail!("The idempotency key must be shorter than {MAX_LENGTH} characters");
        }
        Ok(Self(value))
    }
}

impl From<IdempotencyKey> for String {
    fn from(value: IdempotencyKey) -> Self {
        value.0
    }
}

impl AsRef<str> for IdempotencyKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
