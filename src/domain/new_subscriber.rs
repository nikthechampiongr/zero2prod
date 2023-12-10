use crate::domain::{SubscriberEmail, SubscriberName};

pub struct NewSubscriber {
    pub name: SubscriberName,
    pub email: SubscriberEmail,
}

impl TryInto<NewSubscriber> for crate::Subscription {
    type Error = String;

    fn try_into(self) -> Result<NewSubscriber, Self::Error> {
        Ok(NewSubscriber {
            name: SubscriberName::parse(self.name)?,
            email: SubscriberEmail::parse(self.email)?,
        })
    }
}
