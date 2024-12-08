use actix_web::{web::ReqData, HttpResponse};
use actix_web_flash_messages::FlashMessage;

use crate::{authentication::UserId, session_state::TypedSession, util::see_other};

pub async fn log_out(
    session: TypedSession,
    _: ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    session.log_out();
    FlashMessage::info("You have successfuly logged out.").send();
    Ok(see_other("/login"))
}
