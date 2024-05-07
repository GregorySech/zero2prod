pub enum SubscriberStatus {
    Unsubscribed,
    PendingConfirmation,
    Confirmed,
}

impl SubscriberStatus {
    pub fn parse(s: &str) -> Result<SubscriberStatus, String> {
        match s {
            "pending_confirmation" => Ok(Self::PendingConfirmation),
            "confirmed" => Ok(Self::Confirmed),
            _ => Err(format!("Invalid status representation: {}", s)),
        }
    }
}
