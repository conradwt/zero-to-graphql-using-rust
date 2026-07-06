# Zero to GraphQL Using Rust

The purpose of this example is to provide details as to how one would go about using GraphQL with the Rust Language. Thus, I have created two major sections which should be self explanatory: Quick Installation and Tutorial Installation.

## Getting Started

## Software requirements

- Actix Web 4.14.0 or newer

- Juniper 0.17.1 or newer

- PostgreSQL 18.4 or newer

- Rust 1.96.1 or newer

Note: This tutorial was updated on macOS 26.5.1 (Tahoe).

## Communication

- If you **need help**, use [Stack Overflow](http://stackoverflow.com/questions/tagged/graphql). (Tag 'rust', 'graphql', 'rust-actix', 'actix-web', 'juniper')
- If you'd like to **ask a general question**, use [Stack Overflow](http://stackoverflow.com/questions/tagged/graphql).
- If you **found a bug**, open an issue.
- If you **have a feature request**, open an issue.
- If you **want to contribute**, submit a pull request.

## Quick Installation

1.  clone this repository

    ```zsh
    git clone https://github.com/conradwt/zero-to-graphql-using-rust.git
    ```

2.  change directory location

    ```zsh
    cd zero-to-graphql-using-rust
    ```

3.  create, migrate, and seed the database

    ```zsh
    cargo run -- setup
    ```

4.  start the server

    ```zsh
    cargo run
    ```

5.  navigate to our application within the browser

    ```zsh
    open http://localhost:4000/graphiql
    ```

6.  enter the below GraphQL query on the left side of the browser window

    ```graphql
    {
      person(id: 1) {
        firstName
        lastName
        username
        email
        friends {
          firstName
          lastName
          username
          email
        }
      }
    }
    ```

7.  run the GraphQL query

    ```text
    Control + Enter
    ```

## Tutorial Installation

1.  create the project

    ```zsh
    cargo new zero-to-graphql-using-rust --edition 2024
    ```

2.  switch to the project directory

    ```zsh
    cd zero-to-graphql-using-rust
    ```

3.  create the `config/database.yml` file with the following database environments and credentials:

    ```yaml
    default: &default
      adapter: postgresql
      host: localhost
      port: 5432
      username: postgres
      password: postgres
      pool: 5

    development:
      <<: *default
      database: zero_rust_dev

    test:
      <<: *default
      database: zero_rust_test

    production:
      <<: *default
      url: <%= ENV['DATABASE_URL'] %>
      database: zero_rust_prod
    ```

    You may also optionally create a `.env` file to override the database connection URL for local development:
    ```env
    DATABASE_URL=postgres://postgres:postgres@localhost:5432/zero_rust_dev
    ```

4.  add the required dependencies to `Cargo.toml`:

    ```toml
    [dependencies]
    actix-web = "4.14.0"
    actix-cors = "0.7.1"
    juniper = { version = "0.17.1", features = ["chrono"] }
    sqlx = { version = "0.9.0", features = ["runtime-tokio", "postgres", "chrono", "macros"] }
    tokio = { version = "1.52.3", features = ["full"] }
    serde = { version = "1.0", features = ["derive"] }
    serde_json = "1.0"
    chrono = { version = "0.4", features = ["serde"] }
    dotenvy = "0.15"
    env_logger = "0.11"
    log = "0.4"
    futures-util = "0.3"
    serde_yaml = "0.9.34"
    ```

5.  create database migrations for the tables. We will place these under `migrations/`:

    `migrations/20160730004705_create_people.sql`:

    ```sql
    -- Create people table
    CREATE TABLE people (
        id BIGSERIAL PRIMARY KEY,
        first_name VARCHAR(255) NOT NULL,
        last_name VARCHAR(255) NOT NULL,
        username VARCHAR(255) NOT NULL,
        email VARCHAR(255) NOT NULL,
        inserted_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW()
    );
    ```

    `migrations/20160730024335_create_friendships.sql`:

    ```sql
    -- Create friendships table
    CREATE TABLE friendships (
        id BIGSERIAL PRIMARY KEY,
        person_id BIGINT REFERENCES people(id) ON DELETE CASCADE,
        friend_id BIGINT REFERENCES people(id) ON DELETE CASCADE,
        inserted_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW()
    );

    CREATE INDEX index_friendships_on_person_id ON friendships(person_id);
    CREATE INDEX index_friendships_on_friend_id ON friendships(friend_id);
    ```

6.  create `src/db.rs` to handle database auto-creation, migration execution, and seeding:

    ```rust
    use sqlx::{Connection, PgConnection, PgPool, postgres::PgConnectOptions};
    use std::str::FromStr;

    pub async fn create_database_if_not_exists(database_url: &str) -> Result<(), Box<dyn std::error::Error>> {
        let options = PgConnectOptions::from_str(database_url)?;
        let db_name = options.get_database().ok_or("No database name in DATABASE_URL")?;
        
        // Connect to the default "postgres" database to check and create the target DB
        let admin_options = options.clone().database("postgres");
        let mut conn = match PgConnection::connect_with(&admin_options).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to connect to 'postgres' database: {}. Trying 'template1'...", e);
                let backup_options = options.clone().database("template1");
                PgConnection::connect_with(&backup_options).await?
            }
        };
        
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)"
        )
        .bind(db_name)
        .fetch_one(&mut conn)
        .await?;
        
        if !exists {
            log::info!("Creating database {}...", db_name);
            match sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE \"{}\"", db_name)))
                .execute(&mut conn)
                .await
            {
                Ok(_) => log::info!("Database {} created successfully.", db_name),
                Err(e) => {
                    if let Some(db_err) = e.as_database_error() {
                        let code = db_err.code().as_deref().unwrap_or("");
                        if code == "42P04" || code == "23505" {
                            log::info!("Database {} already exists (ignored concurrent creation).", db_name);
                        } else {
                            return Err(e.into());
                        }
                    } else {
                        return Err(e.into());
                    }
                }
            }
        }
        
        Ok(())
    }

    pub async fn setup_database(database_url: &str) -> Result<PgPool, Box<dyn std::error::Error>> {
        create_database_if_not_exists(database_url).await?;
        
        let pool = PgPool::connect(database_url).await?;
        
        // Run embedded migrations
        sqlx::migrate!("./migrations").run(&pool).await?;
        log::info!("Migrations run successfully.");
        
        Ok(pool)
    }

    pub async fn seed_database(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Seeding database...");
        
        // Truncate tables with cascade to clear out old data
        sqlx::query("TRUNCATE TABLE people RESTART IDENTITY CASCADE")
            .execute(pool)
            .await?;
            
        // Insert people
        let conrad_id: i64 = sqlx::query_scalar(
            "INSERT INTO people (first_name, last_name, email, username) VALUES ($1, $2, $3, $4) RETURNING id"
        )
        .bind("Conrad")
        .bind("Taylor")
        .bind("conradwt@gmail.com")
        .bind("conradwt")
        .fetch_one(pool)
        .await?;
        
        let dhh_id: i64 = sqlx::query_scalar(
            "INSERT INTO people (first_name, last_name, email, username) VALUES ($1, $2, $3, $4) RETURNING id"
        )
        .bind("David")
        .bind("Heinemeier Hansson")
        .bind("dhh@37signals.com")
        .bind("dhh")
        .fetch_one(pool)
        .await?;
        
        let ezra_id: i64 = sqlx::query_scalar(
            "INSERT INTO people (first_name, last_name, email, username) VALUES ($1, $2, $3, $4) RETURNING id"
        )
        .bind("Ezra")
        .bind("Zygmuntowicz")
        .bind("ezra@merbivore.com")
        .bind("ezra")
        .fetch_one(pool)
        .await?;
        
        let matz_id: i64 = sqlx::query_scalar(
            "INSERT INTO people (first_name, last_name, email, username) VALUES ($1, $2, $3, $4) RETURNING id"
        )
        .bind("Yukihiro")
        .bind("Matsumoto")
        .bind("matz@heroku.com")
        .bind("matz")
        .fetch_one(pool)
        .await?;
        
        // Insert friendships
        let friendships = vec![
            (conrad_id, matz_id),
            (dhh_id, ezra_id),
            (dhh_id, matz_id),
            (ezra_id, dhh_id),
            (ezra_id, matz_id),
            (matz_id, conrad_id),
            (matz_id, ezra_id),
            (matz_id, dhh_id),
        ];
        
        for (person_id, friend_id) in friendships {
            sqlx::query("INSERT INTO friendships (person_id, friend_id) VALUES ($1, $2)")
                .bind(person_id)
                .bind(friend_id)
                .execute(pool)
                .await?;
        }
        
        log::info!("Database seeded successfully.");
        Ok(())
    }
    ```

7.  create `src/graphql.rs` to define the GraphQL schema, context, types, and resolvers using Juniper:

    ```rust
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

    pub type Schema = RootNode<'static, Query, Mutation, EmptySubscription<Context>>;

    pub fn create_schema() -> Schema {
        Schema::new(
            Query,
            Mutation,
            EmptySubscription::new(),
        )
    }
    ```

8.  create `src/lib.rs` to declare public modules:

    ```rust
    pub mod db;
    pub mod graphql;
    ```

9.  create `src/main.rs` to start the Actix Web server and handle CLI commands:

    ```rust
    use zero_to_graphql_using_rust::{db, graphql};

    use actix_web::{web, App, HttpResponse, HttpServer, Responder, middleware::Logger};
    use juniper::http::GraphQLRequest;
    use std::env;
    use std::sync::Arc;

    async fn graphql_handler(
        schema: web::Data<Arc<graphql::Schema>>,
        db_pool: web::Data<sqlx::PgPool>,
        data: web::Json<GraphQLRequest>,
    ) -> impl Responder {
        let context = graphql::Context {
            db: db_pool.get_ref().clone(),
        };
        let res = data.execute(&schema, &context).await;
        HttpResponse::Ok().json(res)
    }

    async fn graphiql_handler() -> impl Responder {
        let html = juniper::http::graphiql::graphiql_source("/graphql", None);
        HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html)
    }

    #[actix_web::main]
    async fn main() -> std::io::Result<()> {
        dotenvy::dotenv().ok();
        env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
        
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        
        let args: Vec<String> = env::args().collect();
        if args.len() > 1 {
            match args[1].as_str() {
                "setup" | "--setup" => {
                    if let Err(e) = db::setup_database(&database_url).await {
                        eprintln!("Database setup failed: {}", e);
                        std::process::exit(1);
                    }
                    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
                    if let Err(e) = db::seed_database(&pool).await {
                        eprintln!("Database seeding failed: {}", e);
                        std::process::exit(1);
                    }
                    println!("Database setup and seeding completed successfully.");
                    return Ok(());
                }
                "migrate" | "--migrate" => {
                    if let Err(e) = db::setup_database(&database_url).await {
                        eprintln!("Database migration failed: {}", e);
                        std::process::exit(1);
                    }
                    println!("Database migration completed successfully.");
                    return Ok(());
                }
                "seed" | "--seed" => {
                    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
                    if let Err(e) = db::seed_database(&pool).await {
                        eprintln!("Database seeding failed: {}", e);
                        std::process::exit(1);
                    }
                    println!("Database seeding completed successfully.");
                    return Ok(());
                }
                "help" | "--help" | "-h" => {
                    println!("Zero to GraphQL Using Rust");
                    println!();
                    println!("Usage:");
                    println!("  zero-to-graphql-using-rust [COMMAND]");
                    println!();
                    println!("Commands:");
                    println!("  setup      Create database, run migrations, and seed");
                    println!("  migrate    Run database migrations");
                    println!("  seed       Seed database with initial data");
                    println!("  (none)     Start the Actix Web server");
                    return Ok(());
                }
                _ => {
                    eprintln!("Unknown argument: {}", args[1]);
                    eprintln!("Use --help for usage details.");
                    std::process::exit(1);
                }
            }
        }
        
        log::info!("Connecting to database...");
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to Postgres");
            
        if let Err(e) = sqlx::migrate!("./migrations").run(&pool).await {
            log::warn!("Startup migration run returned error: {}", e);
        }
        
        let schema = Arc::new(graphql::create_schema());
        let port = env::var("PORT").unwrap_or_else(|_| "4000".to_string());
        let bind_address = format!("127.0.0.1:{}", port);
        
        log::info!("Starting GraphQL server at http://{}", bind_address);
        log::info!("GraphiQL playground at http://{}/graphiql", bind_address);
        
        HttpServer::new(move || {
            App::new()
                .wrap(Logger::default())
                .wrap(
                    actix_cors::Cors::default()
                        .allow_any_origin()
                        .allow_any_method()
                        .allow_any_header()
                        .max_age(3600)
                )
                .app_data(web::Data::new(schema.clone()))
                .app_data(web::Data::new(pool.clone()))
                .route("/graphql", web::post().to(graphql_handler))
                .route("/graphiql", web::get().to(graphiql_handler))
        })
        .bind(&bind_address)?
        .run()
        .await
    }
    ```

10. setup the database:

    ```zsh
    cargo run -- setup
    ```

11. start the server:

    ```zsh
    cargo run
    ```

12. navigate to our application within the browser:

    ```zsh
    open http://localhost:4000/graphiql
    ```

13. enter the below GraphQL query on the left side of the browser window:

    ```graphql
    {
      person(id: 1) {
        firstName
        lastName
        username
        email
        friends {
          firstName
          lastName
          username
          email
        }
      }
    }
    ```

14. run the GraphQL query:

    ```text
    Control + Enter
    ```

## Production Setup

Ready to run in production? Please [check our deployment guides](https://actix.rs/docs).

## Actix References

- Official website: https://actix.rs
- Guides: https://actix.rs/docs/getting-started
- Docs: https://actix.rs/docs
- Source: https://github.com/actix/actix-web

## GraphQL References

- Official Website: https://github.com/graphql-rust/juniper

## Support

Bug reports and feature requests can be filed with the rest for the project here:

- [File Bug Reports and Features](https://github.com/conradwt/zero-to-graphql-using-rust/issues)

## License

Zero to GraphQL Using Rust is released under the [MIT license](./LICENSE.md).

## Copyright

Copyright &copy; 2026 Conrad Taylor. All rights reserved.
