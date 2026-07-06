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
