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
    // Load environment variables from .env file
    dotenvy::dotenv().ok();
    
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    
    // Parse CLI arguments for setup/migrate/seed
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
    
    // Default: Start web server
    log::info!("Connecting to database...");
    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to Postgres");
        
    // Ensure migrations are run on startup
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
            // Configure CORS to allow all origins, mirroring cors_plug in Elixir
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
