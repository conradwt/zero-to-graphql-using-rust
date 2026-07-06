use juniper::{EmptySubscription, FieldResult, RootNode, ID};
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Person {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
    pub username: String,
    pub email: String,
    pub inserted_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

pub struct Context {
    pub db: PgPool,
}

impl juniper::Context for Context {}

#[juniper::graphql_object(context = Context, description = "a person")]
impl Person {
    #[graphql(description = "unique identifier for the person")]
    fn id(&self) -> String {
        self.id.to_string()
    }

    #[graphql(description = "first name of a person")]
    fn first_name(&self) -> &str {
        &self.first_name
    }

    #[graphql(description = "last name of a person")]
    fn last_name(&self) -> &str {
        &self.last_name
    }

    #[graphql(description = "username of a person")]
    fn username(&self) -> &str {
        &self.username
    }

    #[graphql(description = "email of a person")]
    fn email(&self) -> &str {
        &self.email
    }

    #[graphql(description = "a list of friends for our person")]
    async fn friends(&self, context: &Context) -> FieldResult<Vec<Person>> {
        let friends = sqlx::query_as::<_, Person>(
            "SELECT p.id, p.first_name, p.last_name, p.username, p.email, p.inserted_at, p.updated_at \
             FROM people p \
             JOIN friendships f ON f.friend_id = p.id \
             WHERE f.person_id = $1"
        )
        .bind(self.id)
        .fetch_all(&context.db)
        .await?;
        
        Ok(friends)
    }
}

#[derive(juniper::GraphQLInputObject)]
#[graphql(description = "a person input")]
pub struct PersonInput {
    pub first_name: String,
    pub last_name: String,
    pub username: String,
    pub email: String,
}

pub struct Query;

#[juniper::graphql_object(context = Context)]
impl Query {
    async fn person(context: &Context, id: ID) -> FieldResult<Person> {
        let id_i64 = id.parse::<i64>().map_err(|_| {
            juniper::FieldError::new(
                format!("Invalid ID format: {}", id),
                juniper::Value::null()
            )
        })?;
        
        let person = sqlx::query_as::<_, Person>(
            "SELECT id, first_name, last_name, username, email, inserted_at, updated_at \
             FROM people WHERE id = $1"
        )
        .bind(id_i64)
        .fetch_optional(&context.db)
        .await?;
        
        match person {
            Some(p) => Ok(p),
            None => Err(juniper::FieldError::new(
                format!("Person id {} not found", id),
                juniper::Value::null()
            )),
        }
    }

    async fn people(context: &Context, ids: Option<Vec<ID>>) -> FieldResult<Vec<Person>> {
        let ids_vec: Vec<i64> = match ids {
            Some(vec) => vec.into_iter()
                .filter_map(|id| id.parse::<i64>().ok())
                .collect(),
            None => Vec::new(),
        };
        
        let people = if ids_vec.is_empty() {
            sqlx::query_as::<_, Person>(
                "SELECT id, first_name, last_name, username, email, inserted_at, updated_at \
                 FROM people ORDER BY id ASC"
            )
            .fetch_all(&context.db)
            .await?
        } else {
            sqlx::query_as::<_, Person>(
                "SELECT id, first_name, last_name, username, email, inserted_at, updated_at \
                 FROM people WHERE id = ANY($1) ORDER BY id ASC"
            )
            .bind(&ids_vec)
            .fetch_all(&context.db)
            .await?
        };
        
        Ok(people)
    }
}

pub struct Mutation;

#[juniper::graphql_object(context = Context)]
impl Mutation {
    async fn create_person(context: &Context, input: PersonInput) -> FieldResult<Person> {
        let person = sqlx::query_as::<_, Person>(
            "INSERT INTO people (first_name, last_name, email, username) VALUES ($1, $2, $3, $4) \
             RETURNING id, first_name, last_name, email, username, inserted_at, updated_at"
        )
        .bind(input.first_name)
        .bind(input.last_name)
        .bind(input.email)
        .bind(input.username)
        .fetch_one(&context.db)
        .await?;
        
        Ok(person)
    }
}

pub type Schema = RootNode<Query, Mutation, EmptySubscription<Context>>;

pub fn create_schema() -> Schema {
    Schema::new(
        Query,
        Mutation,
        EmptySubscription::new(),
    )
}
