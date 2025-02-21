use actix_web::{body::to_bytes, HttpResponse};
use reqwest::StatusCode;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::IdempotencyKey;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
pub struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(HttpResponse),
}

pub async fn try_processing(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<NextAction, anyhow::Error> {
    let mut transaction = pool.begin().await?;
    let query = sqlx::query!(
        r#"
        INSERT INTO idempotency(
            user_id,
            idempotency_key,
            created_at
        )
        VALUES ($1, $2, now())
        ON CONFLICT DO NOTHING
        "#,
        user_id,
        idempotency_key.as_ref()
    );

    let modified = transaction.execute(query).await?.rows_affected() > 0;

    if modified {
        Ok(NextAction::StartProcessing(transaction))
    } else {
        let saved_response = get_saved_response(pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Expected a saved response, but failed to get it"))?;
        Ok(NextAction::ReturnSavedResponse(saved_response))
    }
}

pub async fn save_response(
    mut transaction: Transaction<'static, Postgres>,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
    response: HttpResponse,
) -> Result<HttpResponse, anyhow::Error> {
    let status = response.status().as_u16() as i16;

    let (response_headers, body) = response.into_parts();
    let headers = {
        let mut v = Vec::with_capacity(response_headers.headers().len());

        for (name, value) in response_headers.headers() {
            v.push(HeaderPairRecord {
                name: name.as_str().to_string(),
                value: value.as_bytes().to_owned(),
            });
        }
        v
    };
    let body = to_bytes(body).await.map_err(|e| anyhow::anyhow!("{e}"))?;

    let query = sqlx::query!(
        r#"UPDATE idempotency SET
            response_status_code = $3,
            response_headers = $4,
            response_body = $5
        WHERE user_id = $1 AND
        idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
        status,
        headers as Vec<HeaderPairRecord>,
        body.as_ref()
    );
    transaction.execute(query).await?;

    transaction.commit().await?;

    let response = response_headers.set_body(body).map_into_boxed_body();
    Ok(response)
}

pub async fn get_saved_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<Option<HttpResponse>, anyhow::Error> {
    let saved_responses = sqlx::query!(
        r#"
        SELECT 
            response_status_code as "response_status_code!",
            response_headers as "response_headers!: Vec<HeaderPairRecord>",
            response_body as "response_body!"
        FROM idempotency
        WHERE 
            user_id = $1 AND 
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .fetch_optional(pool)
    .await?;

    if let Some(r) = saved_responses {
        let status_code = StatusCode::from_u16(r.response_status_code.try_into()?)?;

        let mut response = HttpResponse::build(status_code);

        for HeaderPairRecord { name, value } in r.response_headers {
            response.append_header((name, value));
        }
        return Ok(Some(response.body(r.response_body)));
    }

    Ok(None)
}
