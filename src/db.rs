use sqlx::{Connection, PgConnection, PgPool, postgres::{PgConnectOptions, PgPoolOptions}};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, serde::Deserialize, Clone)]
pub struct DbConfig {
    pub adapter: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
    pub pool: Option<u32>,
    pub url: Option<String>,
}

fn expand_env_vars(text: &str) -> String {
    let mut result = text.to_string();
    
    // Replace <%= ENV['VAR'] %> (Ruby style)
    while let Some(start_idx) = result.find("<%= ENV['") {
        if let Some(end_idx) = result[start_idx..].find("'] %>") {
            let actual_end_idx = start_idx + end_idx;
            let var_name = &result[start_idx + 9..actual_end_idx];
            let val = env::var(var_name).unwrap_or_default();
            result.replace_range(start_idx..(actual_end_idx + 5), &val);
        } else {
            break;
        }
    }
    
    // Replace ${VAR} (Standard env style)
    while let Some(start_idx) = result.find("${") {
        if let Some(end_idx) = result[start_idx..].find("}") {
            let actual_end_idx = start_idx + end_idx;
            let var_name = &result[start_idx + 2..actual_end_idx];
            let val = env::var(var_name).unwrap_or_default();
            result.replace_range(start_idx..(actual_end_idx + 1), &val);
        } else {
            break;
        }
    }
    
    result
}

impl DbConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let env_name = env::var("RUST_ENV")
            .or_else(|_| env::var("APP_ENV"))
            .unwrap_or_else(|_| "development".to_string());
            
        let config_path = Path::new("config/database.yml");
        if !config_path.exists() {
            return Err("config/database.yml not found".into());
        }
        
        let content = fs::read_to_string(config_path)?;
        let expanded = expand_env_vars(&content);
        
        let yml: HashMap<String, DbConfig> = serde_yaml::from_str(&expanded)?;
        
        let config = yml.get(&env_name)
            .ok_or_else(|| format!("Environment '{}' not found in database.yml", env_name))?
            .clone();
            
        Ok(config)
    }

    pub fn to_connection_string(&self) -> String {
        if let Some(ref url) = self.url {
            if !url.is_empty() {
                return url.clone();
            }
        }
        
        let host = self.host.as_deref().unwrap_or("localhost");
        let port = self.port.unwrap_or(5432);
        let username = self.username.as_deref().unwrap_or("postgres");
        let password = self.password.as_deref().unwrap_or("");
        let database = self.database.as_deref().unwrap_or("");
        
        if password.is_empty() {
            format!("postgres://{}@{}:{}/{}", username, host, port, database)
        } else {
            format!("postgres://{}:{}@{}:{}/{}", username, password, host, port, database)
        }
    }
}

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
                    if db_err.code().map(|c| c == "42P04" || c == "23505").unwrap_or(false) {
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

pub async fn setup_database_with_config(config: &DbConfig) -> Result<PgPool, Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| config.to_connection_string());
    
    create_database_if_not_exists(&database_url).await?;
    
    let pool_size = config.pool.unwrap_or(5);
    let pool = PgPoolOptions::new()
        .max_connections(pool_size)
        .connect(&database_url)
        .await?;
        
    // Run embedded migrations
    sqlx::migrate!("./db/migrations").run(&pool).await?;
    log::info!("Migrations run successfully.");
    
    Ok(pool)
}

pub async fn setup_database() -> Result<PgPool, Box<dyn std::error::Error>> {
    let config = DbConfig::load()?;
    setup_database_with_config(&config).await
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
