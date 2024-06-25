#[derive(Debug)]
pub struct IdempotencyKey(String);

impl TryFrom<String> for IdempotencyKey {
    type Error = anyhow::Error;

    fn try_from(string: String) -> Result<Self, Self::Error> {
        if string.is_empty() {
            anyhow::bail!("The idempotency key cannot be empty!")
        }

        let max_length = 50;
        if string.len() >= max_length {
            anyhow::bail!("The idempotency key must be shorted than {max_length} characters.");
        }

        Ok(Self(string))
    }
}

impl From<IdempotencyKey> for String {
    fn from(key: IdempotencyKey) -> Self {
        key.0
    }
}

impl AsRef<str> for IdempotencyKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
