use zero_to_graphql_using_rust::{db, graphql};
use std::env;
use sqlx::PgPool;
use juniper::{graphql_value, Variables};

async fn setup_test_db() -> PgPool {
    dotenvy::dotenv().ok();
    let db_url = env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/zero_rust_test".to_string());
        
    db::create_database_if_not_exists(&db_url).await.unwrap();
    let pool = db::setup_database(&db_url).await.unwrap();
    db::seed_database(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn test_get_person_by_id() {
    let pool = setup_test_db().await;
    let schema = graphql::create_schema();
    let context = graphql::Context { db: pool };
    
    let query = r#"
      query GetPerson($personId: ID!) {
        person(id: $personId) {
          email
        }
      }
    "#;
    
    let mut vars = Variables::new();
    vars.insert("personId".to_string(), juniper::InputValue::scalar("1"));
    
    let (res, errors) = juniper::execute(query, None, &schema, &vars, &context)
        .await
        .unwrap();
        
    assert!(errors.is_empty(), "GraphQL errors occurred: {:?}", errors);
    
    let expected = graphql_value!({
        "person": {
            "email": "conradwt@gmail.com"
        }
    });
    
    assert_eq!(res, expected);
}

#[tokio::test]
async fn test_get_people_by_ids() {
    let pool = setup_test_db().await;
    let schema = graphql::create_schema();
    let context = graphql::Context { db: pool };
    
    let query = r#"
      query GetPeople($ids: [ID!]) {
        people(ids: $ids) {
          firstName
          lastName
        }
      }
    "#;
    
    let mut vars = Variables::new();
    vars.insert(
        "ids".to_string(),
        juniper::InputValue::list(vec![
            juniper::InputValue::scalar("1"),
            juniper::InputValue::scalar("2"),
        ]),
    );
    
    let (res, errors) = juniper::execute(query, None, &schema, &vars, &context)
        .await
        .unwrap();
        
    assert!(errors.is_empty(), "GraphQL errors occurred: {:?}", errors);
    
    let expected = graphql_value!({
        "people": [
            {
                "firstName": "Conrad",
                "lastName": "Taylor"
            },
            {
                "firstName": "David",
                "lastName": "Heinemeier Hansson"
            }
        ]
    });
    
    assert_eq!(res, expected);
}

#[tokio::test]
async fn test_get_people() {
    let pool = setup_test_db().await;
    let schema = graphql::create_schema();
    let context = graphql::Context { db: pool };
    
    let query = r#"
      query GetPeople {
        people {
          firstName
          lastName
        }
      }
    "#;
    
    let (res, errors) = juniper::execute(query, None, &schema, &Variables::new(), &context)
        .await
        .unwrap();
        
    assert!(errors.is_empty(), "GraphQL errors occurred: {:?}", errors);
    
    let expected = graphql_value!({
        "people": [
            {
                "firstName": "Conrad",
                "lastName": "Taylor"
            },
            {
                "firstName": "David",
                "lastName": "Heinemeier Hansson"
            },
            {
                "firstName": "Ezra",
                "lastName": "Zygmuntowicz"
            },
            {
                "firstName": "Yukihiro",
                "lastName": "Matsumoto"
            }
        ]
    });
    
    assert_eq!(res, expected);
}

#[tokio::test]
async fn test_create_person() {
    let pool = setup_test_db().await;
    let schema = graphql::create_schema();
    let context = graphql::Context { db: pool };
    
    let query = r#"
      mutation CreatePerson($input: PersonInput!) {
        createPerson(input: $input) {
          firstName
          lastName
          username
          email
        }
      }
    "#;
    
    let mut vars = Variables::new();
    vars.insert(
        "input".to_string(),
        juniper::graphql_input_value!({
            "firstName": "Matz",
            "lastName": "Matsumoto",
            "username": "matz",
            "email": "matz@heroku.com"
        }),
    );
    
    let (res, errors) = juniper::execute(query, None, &schema, &vars, &context)
        .await
        .unwrap();
        
    assert!(errors.is_empty(), "GraphQL errors occurred: {:?}", errors);
    
    let expected = graphql_value!({
        "createPerson": {
            "firstName": "Matz",
            "lastName": "Matsumoto",
            "username": "matz",
            "email": "matz@heroku.com"
        }
    });
    
    assert_eq!(res, expected);
}
