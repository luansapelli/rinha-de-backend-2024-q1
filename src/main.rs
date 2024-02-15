use std::env;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chrono;
use sqlx::{Executor, PgPool, Row};
use tokio::net::TcpListener;

#[derive(sqlx::FromRow)]
struct Client {
    limit_value: i32,
    balance: i32,
}

#[derive(sqlx::FromRow)]
struct Transactions {
    client_id: i32,
    value: Option<i32>,
    tran_type: Option<String>,
    description: Option<String>,
    created_at: Option<String>,
    limit_value: i32,
    balance: i32,
}

#[derive(serde::Deserialize, Debug)]
struct TransactionRequest {
    tipo: Option<String>,
    valor: Option<i32>,
    descricao: Option<String>,
}

#[derive(serde::Serialize)]
struct TransactionResponse {
    limite: i32,
    saldo: i32,
}

#[derive(serde::Serialize)]
pub struct AccountStatementResponse {
    pub saldo: Balance,
    pub ultimas_transacoes: Vec<TransactionInfo>,
}

#[derive(serde::Serialize)]
pub struct Balance {
    pub total: i32,
    pub data_extrato: String,
    pub limite: i32,
}

#[derive(serde::Serialize)]
pub struct TransactionInfo {
    pub valor: i32,
    pub tipo: String,
    pub descricao: String,
    pub realizada_em: String,
}

async fn do_transaction(
    path: web::Path<(i16,)>,
    bytes: web::Bytes,
    db_pool: web::Data<PgPool>,
) -> impl Responder {
    if path.0 > 5 {
        return HttpResponse::NotFound().finish();
    }

    let transaction  = match serde_json::from_slice::<TransactionRequest>(&bytes) {
        Ok(transaction) => transaction,
        Err(_) => {
            return HttpResponse::UnprocessableEntity().finish()
        },
    };

    let transaction_description = match transaction.descricao {
        Some(description) => {
            if description.len() < 1 || description.len() > 10 {
            return HttpResponse::UnprocessableEntity().finish();
            }

            description
        },
        None => return HttpResponse::UnprocessableEntity().finish(),
    };


    let transaction_type = match transaction.tipo {
        Some(tran_type) => {
            if tran_type != "c" && tran_type != "d" {
                return HttpResponse::UnprocessableEntity().finish();
            }

            tran_type
        },
        None => return HttpResponse::UnprocessableEntity().finish(),
    };

    let transaction_value = match transaction.valor {
        Some(value) => {
            if value < 0 {
                return HttpResponse::UnprocessableEntity().finish();
            }

            value
        },
        None => return HttpResponse::UnprocessableEntity().finish(),
    };

    let mut db_transaction = db_pool.begin().await.expect("Can not start transaction");
    match sqlx::query(r#"
                SELECT * FROM process_transaction($1, $2, $3, $4, $5) AS result;
            "#)
        .bind(path.0)
        .bind(transaction_value)
        .bind(transaction_type)
        .bind(transaction_description)
        .bind(chrono::Utc::now().to_rfc3339())
        .fetch_one(&mut *db_transaction)
        .await
    {
        Ok(result) => {
            db_transaction.commit().await.expect("Can not commit transaction");
            HttpResponse::Ok().json(TransactionResponse {
                limite: result.get(0),
                saldo: result.get(1),
            })
        }
        Err(_) => {
            db_transaction.rollback().await.expect("Can not rollback transaction");
            HttpResponse::InternalServerError().finish()
        }
    }
}


const FETCH_ACCOUNT_STATEMENT_QUERY: &str = r#"
    SELECT
        c.id AS client_id,
        t.value,
        t.tran_type,
        t.description,
        t.created_at,
        c.limit_value,
        c.balance
    FROM
        clients c
    LEFT JOIN
        transactions t ON c.id = t.client_id
    WHERE
        c.id = $1
    ORDER BY
        t.created_at DESC
    LIMIT
        10;
"#;

async fn fetch_account_statement(
    path: web::Path<(i16,)>,
    db_pool: web::Data<PgPool>,
) -> impl Responder {
    if path.0 > 5 {
        return HttpResponse::NotFound().finish();
    }

    match sqlx::query_as::<_, Transactions>(FETCH_ACCOUNT_STATEMENT_QUERY)
    .bind(path.0)
    .fetch_all(db_pool.get_ref())
    .await
    {
        Ok(transactions) => {
            if transactions.is_empty() {
                return HttpResponse::Ok().json(AccountStatementResponse {
                    saldo: Balance {
                        total: transactions[0].balance,
                        data_extrato: chrono::Utc::now().to_rfc3339(),
                        limite: transactions[0].limit_value,
                    },
                    ultimas_transacoes: vec![],
                });
            }

            HttpResponse::Ok().json(AccountStatementResponse {
                saldo: Balance {
                    total: transactions[0].balance,
                    data_extrato: chrono::Utc::now().to_rfc3339(),
                    limite: transactions[0].limit_value,
                },
                ultimas_transacoes: transactions
                    .iter()
                    .map(|t| TransactionInfo {
                        valor: t.value.clone().unwrap_or(0),
                        tipo: t.tran_type.clone().unwrap_or("".to_string()),
                        descricao: t.description.clone().unwrap_or("".to_string()),
                        realizada_em: t.created_at.clone().unwrap_or("".to_string()),
                    })
                    .collect(),
            })
        },
        Err(_) => {
            return HttpResponse::NotFound().finish()
        },
    }
}

#[tokio::main]
async fn main() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db_pool = match PgPool::connect(database_url.as_str())
        .await
    {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("Can not connect to database: {:?}", err);
            return;
        }
    };

    HttpServer::new(move || {
        App::new()
            .route("/clientes/{id}/transacoes", web::post().to(do_transaction))
            .route(
                "/clientes/{id}/extrato",
                web::get().to(fetch_account_statement),
            )
            .app_data(web::Data::new(db_pool.clone()))
    })
    .bind("0.0.0.0:8000")
    .expect("Can not bind to port 8000")
    .run()
    .await
    .expect("Can not start server");
}
