use anyhow::Context;
use sqlx::{Executor, PgPool, Postgres, Row, Transaction};
use std::time::Duration;
use tracing::{Span, field::display};
use uuid::Uuid;

use crate::{
    configuration::Settings, domain::SubscriberEmail, email_client::EmailClient,
    startup::get_connection_pool,
};

pub enum TaskOutcome {
    TaskComplete,
    QueueEmpty,
}

pub async fn run_workers_until_stopped(configuration: Settings) -> Result<(), anyhow::Error> {
    let pool = get_connection_pool(&configuration.database);
    let email_client = configuration.email_client.client();
    worker_loop(pool, email_client).await
}

async fn worker_loop(pool: PgPool, email_client: EmailClient) -> Result<(), anyhow::Error> {
    loop {
        match try_execute_task(&pool, &email_client).await {
            Ok(TaskOutcome::QueueEmpty) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            Ok(TaskOutcome::TaskComplete) => {}
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

#[tracing::instrument(name="Try execute task", skip_all, fields(newsletter_issue_id=tracing::field::Empty, subscriber_email=tracing::field::Empty), err)]
pub async fn try_execute_task(
    pool: &PgPool,
    email_client: &EmailClient,
) -> Result<TaskOutcome, anyhow::Error> {
    if let Some((tx, issue_id, email)) = dequeue_task(pool).await? {
        Span::current()
            .record("newsletter_issue_id", display(issue_id))
            .record("subscriber_email", display(&email));

        match SubscriberEmail::parse(email.clone()) {
            Ok(email) => {
                let issue = get_issue(pool, issue_id).await?;
                if let Err(e) = email_client
                    .send_email(
                        &email,
                        &issue.title,
                        &issue.html_content,
                        &issue.text_content,
                    )
                    .await
                    .with_context(|| "Failed to send newsletter issue to confirmed subscriber")
                {
                    tracing::error!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "Failed to deliver issue to subscriber. \
                        Skipping...
                        "
                    )
                }
            }
            Err(e) => tracing::warn!(error.cause_chain = ?e,
                "Skipping a confirmed subscriber\
                Their stored contact information is invalid"),
        }
        delete_task(tx, issue_id, &email).await?;
        return Ok(TaskOutcome::TaskComplete);
    }

    Ok(TaskOutcome::QueueEmpty)
}

#[tracing::instrument(skip_all, name = "Dequeue task")]
async fn dequeue_task(
    pool: &PgPool,
) -> Result<Option<(Transaction<'_, Postgres>, Uuid, String)>, anyhow::Error> {
    let mut transaction = pool.begin().await?;

    let res = sqlx::query!(
        r#"
        SELECT 
            newsletter_issue_id,
            subscriber_email
        FROM issue_delivery_queue
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1
        "#,
    )
    .fetch_optional(&mut *transaction)
    .await?;

    // "Nik why are you manually get()ting stuff?"
    // sqlx transactions are bugged and if you fetch optional the way god intended  with query!
    // then sqlx creates a PgRow.
    if let Some(item) = res {
        Ok(Some((
            transaction,
            item.newsletter_issue_id,
            item.subscriber_email,
        )))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(skip_all, name = "Delete task")]
async fn delete_task(
    mut tx: Transaction<'_, Postgres>,
    id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    let query = sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue 
        WHERE 
            newsletter_issue_id = $1 
            AND subscriber_email = $2"#,
        id,
        email
    );

    tx.execute(query).await?;
    tx.commit().await?;
    Ok(())
}

async fn get_issue(pool: &PgPool, issue_id: Uuid) -> Result<NewsletterIssue, anyhow::Error> {
    Ok(sqlx::query_as!(
        NewsletterIssue,
        r#"
            SELECT title,html_content,text_content 
            FROM newsletter_issues 
            WHERE
                newsletter_issue_id = $1
            "#,
        issue_id
    )
    .fetch_one(pool)
    .await?)
}

struct NewsletterIssue {
    title: String,
    html_content: String,
    text_content: String,
}
