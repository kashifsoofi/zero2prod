use crate::domain::{NewSubscriber, SubscriberName, SubscriberEmail};
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct SubscriptionRequest {
    email: String,
    name: String,
}

impl TryFrom<web::Json<SubscriptionRequest>> for NewSubscriber {
    type Error = String;
    
    fn try_from(value: web::Json<SubscriptionRequest>) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name.clone())?;
        let email = SubscriberEmail::parse(value.email.clone())?;
        Ok(Self { email, name })
    }
} 

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(request, pool),
    fields(
        subscriber_email = %request.email,
        subscriber_name = %request.name,
    )
)]
pub async fn subscribe(
    request: web::Json<SubscriptionRequest>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let new_subscriber = match request.try_into() {
        Ok(form) => form,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    match insert_subscriber(&pool, &new_subscriber).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_e) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, pool)
)]
pub async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
