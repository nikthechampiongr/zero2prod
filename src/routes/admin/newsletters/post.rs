use serde::Deserialize;
use sqlx::Executor;

use actix_web::HttpResponse;
use actix_web::web;
use actix_web_flash_messages::FlashMessage;
use sqlx::PgPool;
use sqlx::Postgres;
use sqlx::Transaction;
use uuid::Uuid;

use crate::authentication::UserId;
use crate::idempotency::IdempotencyKey;
use crate::idempotency::NextAction;
use crate::idempotency::save_response;
use crate::idempotency::try_processing;
use crate::util::e400;
use crate::util::e500;
use crate::util::see_other;

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    html: String,
    text: String,
    idempotency_key: String,
}

#[tracing::instrument(
    name = "Publish newsletter to confirmed subscriber",
    skip_all,
    fields(userid=%&*user_id)
)]
pub async fn post_newsletters(
    body: web::Form<BodyData>,
    pg_pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();

    let BodyData {
        title,
        html,
        text,
        idempotency_key,
    } = body.0;

    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;

    let mut transaction = match try_processing(&pg_pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(transaction) => transaction,
        NextAction::ReturnSavedResponse(res) => {
            success();
            return Ok(res);
        }
    };

    let issue_id = insert_newsletter_issue(&mut transaction, &title, &html, &text)
        .await
        .map_err(e500)?;

    enque_delivery_tasks(&mut transaction, issue_id)
        .await
        .map_err(e500)?;

    success();
    let response = see_other("/admin/newsletters");
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
}

pub fn success() {
    FlashMessage::info("The newsletter has been published!".to_string()).send();
}

#[tracing::instrument(name = "Insert newsletter issue", skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    html_content: &str,
    text_content: &str,
) -> Result<Uuid, anyhow::Error> {
    let newsletter_issue_id = Uuid::new_v4();

    let query = sqlx::query!(
        r#"
        INSERT INTO newsletter_issues(
            newsletter_issue_id,
            title,
            html_content,
            text_content,
            published_at
            )
        VALUES($1, $2, $3, $4, now())
        "#,
        newsletter_issue_id,
        title,
        html_content,
        text_content
    );
    transaction.execute(query).await?;
    Ok(newsletter_issue_id)
}

async fn enque_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue(
            newsletter_issue_id,
            subscriber_email
            )
            SELECT $1, email FROM subscriptions
            WHERE status = 'confirmed'
        "#,
        newsletter_issue_id
    );

    transaction.execute(query).await?;
    Ok(())
}
