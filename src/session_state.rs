use std::future::{Ready, ready};

use actix_session::{Session, SessionExt};
use actix_web::FromRequest;
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &str = "user_id";

    pub fn renew(&self) {
        self.0.renew();
    }

    pub fn insert_user_id(&self, user_id: Uuid) -> Result<(), actix_session::SessionInsertError> {
        self.0.insert(Self::USER_ID_KEY, user_id)
    }

    pub fn get_user_id(&self) -> Result<Option<Uuid>, actix_session::SessionGetError> {
        self.0.get(Self::USER_ID_KEY)
    }

    pub fn log_out(&self) {
        self.0.purge();
    }
}

impl FromRequest for TypedSession {
    type Error = <Session as FromRequest>::Error;

    type Future = Ready<Result<TypedSession, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        ready(Ok(TypedSession(req.get_session())))
    }
}
