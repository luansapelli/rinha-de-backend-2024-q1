use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chrono;
use sqlx::PgPool;

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

    let description = match &transaction.descricao {
        Some(description) => {
            if description.len() < 1 || description.len() > 10 {
                return HttpResponse::UnprocessableEntity().finish();
            }

            description
        },
        None => {
            return HttpResponse::UnprocessableEntity().finish();
        }
    };

    let transaction_type = match transaction.tipo.as_deref() {
        Some("c") => "c",
        Some("d") => "d",
        _ => return HttpResponse::UnprocessableEntity().finish(),
    };

    let value = match transaction.valor {
        Some(value) => {
            if value < 1 {
                return HttpResponse::UnprocessableEntity().finish();
            }

            value
        },
        None => {
            return HttpResponse::UnprocessableEntity().finish();
        }
    };

    let client = match sqlx::query_as::<_, Client>("SELECT * FROM clients WHERE id = $1")
        .bind(path.0)
        .fetch_one(db_pool.get_ref())
        .await
    {
        Ok(client) => client,
        Err(_) => return HttpResponse::NotFound().finish(),
    };

    let new_balance = match transaction_type {
        "c" => client.balance + value,
        "d" => {
            let potential_balance = client.balance - value;
            if potential_balance < -client.limit_value {
                return HttpResponse::UnprocessableEntity().finish();
            }

            potential_balance
        }
        _ => return HttpResponse::UnprocessableEntity().finish(),
    };

    let mut db_transaction = db_pool.begin().await.expect("Can not start transaction");
    match sqlx::query(r#"INSERT INTO transactions (client_id, value, tran_type, description, created_at) VALUES ($1, $2, $3, $4, $5)"#)
        .bind(path.0)
        .bind(value)
        .bind(transaction_type)
        .bind(description)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&mut *db_transaction)
        .await
    {
        Ok(_) => {
            match sqlx::query(r#"UPDATE clients SET balance = $1 WHERE id = $2"#)
                .bind(new_balance)
                .bind(path.0)
                .execute(&mut *db_transaction)
                .await
            {
                Ok(_) => {},
                Err(_) => {
                    db_transaction.rollback().await.expect("Can not rollback transaction");
                    return HttpResponse::InternalServerError().finish();
                }
            }

            db_transaction.commit().await.expect("Can not commit transaction");
        }
        Err(_) => {
            db_transaction.rollback().await.expect("Can not rollback transaction");
            return HttpResponse::InternalServerError().finish();
        }
    }

    HttpResponse::Ok().json(TransactionResponse {
        limite: client.limit_value,
        saldo: new_balance,
    })
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
    let db_pool = PgPool::connect("postgres://postgres:password@localhost/rinha")
        .await
        .expect("Can not connect to database");

    HttpServer::new(move || {
        App::new()
            .route("/clientes/{id}/transacoes", web::post().to(do_transaction))
            .route(
                "/clientes/{id}/extrato",
                web::get().to(fetch_account_statement),
            )
            .app_data(web::Data::new(db_pool.clone()))
    })
    .bind("localhost:9999")
    .expect("Can not bind to port 9999")
    .run()
    .await
    .expect("Can not start server");
}
