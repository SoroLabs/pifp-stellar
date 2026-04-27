use async_graphql::*;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use futures_util::{Stream, StreamExt};
use sqlx::{PgPool, postgres::PgNotification};
use std::sync::Arc;
use crate::graphql::model::{Project, Event};
use crate::db;

pub mod model;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn projects(
        &self,
        ctx: &Context<'_>,
        status: Option<String>,
        creator: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Project>> {
        let pool = ctx.data::<PgPool>()?;
        let records = db::list_projects(
            pool,
            status,
            creator,
            None,
            limit.unwrap_or(10),
            offset.unwrap_or(0),
        ).await?;
        Ok(records.into_iter().map(Project::from).collect())
    }

    async fn project(&self, ctx: &Context<'_>, id: String) -> Result<Option<Project>> {
        let pool = ctx.data::<PgPool>()?;
        // Simple mock for specific project fetch if not in db.rs
        let projects = db::list_projects(pool, None, None, None, 1, 0).await?;
        Ok(projects.into_iter().find(|p| p.project_id == id).map(Project::from))
    }

    async fn events(&self, ctx: &Context<'_>, limit: Option<i64>) -> Result<Vec<Event>> {
        let pool = ctx.data::<PgPool>()?;
        let records = db::get_all_events(pool).await?;
        Ok(records.into_iter().take(limit.unwrap_or(20) as usize).map(Event::from).collect())
    }
}

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    async fn activity_feed(&self, ctx: &Context<'_>) -> impl Stream<Item = Event> {
        let pool = ctx.data::<PgPool>().cloned().unwrap();
        let mut listener = sqlx::postgres::PgListener::connect_with(&pool).await.unwrap();
        listener.listen("events").await.unwrap();

        listener.into_stream().map(|notification| {
            let notification = notification.unwrap();
            let payload: crate::events::PifpEvent = serde_json::from_str(notification.payload()).unwrap();
            
            // Convert PifpEvent to EventRecord mock or directly to GraphQL Event
            Event {
                id: ID::from(0), // Simplified for subscription
                event_type: payload.event_type,
                project_id: payload.project_id,
                actor: payload.actor,
                amount: payload.amount,
                ledger: payload.ledger,
                timestamp: payload.timestamp,
                contract_id: payload.contract_id,
                tx_hash: payload.tx_hash,
                extra_data: payload.extra_data,
                created_at: chrono::Utc::now().timestamp(),
            }
        })
    }
}

pub type AppSchema = Schema<QueryRoot, EmptyMutation, SubscriptionRoot>;

async fn graphql_handler(
    State(schema): State<AppSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn graphiql() -> impl IntoResponse {
    Html(
        GraphiQLSource::build()
            .endpoint("/graphql")
            .subscription_endpoint("/graphql/ws")
            .finish(),
    )
}

pub fn router(pool: PgPool) -> Router {
    let schema = Schema::build(QueryRoot, EmptyMutation, SubscriptionRoot)
        .data(pool)
        .finish();

    Router::new()
        .route("/graphql", get(graphiql).post(graphql_handler))
        .route("/graphql/ws", GraphQLSubscription::new(schema.clone()))
        .with_state(schema)
}
