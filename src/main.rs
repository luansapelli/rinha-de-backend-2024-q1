use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chrono;
use sqlx::{PgPool};

#[derive(serde::Deserialize)]
struct TransactionRequest {
    tipo: String,
    valor: i64,
    descricao: String,
}

#[derive(serde::Serialize)]
struct TransactionResponse {
    limite: i64,
    saldo: i64,
}

#[derive(serde::Serialize)]
pub struct AccountStatementResponse {
    pub saldo: Balance,
    pub ultimas_transacoes: Vec<TransactionInfo>,
}

#[derive(serde::Serialize)]
pub struct Balance {
    pub total: i64,
    pub data_extrato: String,
    pub limite: i64,
}

#[derive(serde::Serialize)]
pub struct TransactionInfo {
    pub valor: i64,
    pub tipo: String,
    pub descricao: String,
    pub realizada_em: String,
}

async fn do_transaction(path: web::Path<(i16,)>, transaction: web::Json<TransactionRequest>, db_pool: web::Data<PgPool>) -> impl Responder {
    if path.0 < 0 || path.0 > 5 {
        return HttpResponse::NotFound().finish();
    }

    if transaction.tipo != "c" && transaction.tipo != "d" {
        return HttpResponse::BadRequest().finish();
    }

    if transaction.descricao.len() < 1 || transaction.descricao.len() > 10 {
        return HttpResponse::BadRequest().finish();
    }

    let mut db_transaction = db_pool.begin().await.expect("Can not start transaction");
    match sqlx::query(r#"INSERT INTO transactions (client_id, value, "type", description) VALUES ($1, $2, $3, $4)"#)
        .bind(path.0)
        .bind(transaction.valor)
        .bind(transaction.tipo.as_str())
        .bind(transaction.descricao.as_str())
        .execute(&mut *db_transaction)
        .await
    {
        Ok(_) => {
            db_transaction.commit().await.expect("Can not commit transaction");
        }
        Err(_) => {
            db_transaction.rollback().await.expect("Can not rollback transaction");
            return HttpResponse::InternalServerError().finish();
        }
    }

    HttpResponse::Ok().json(TransactionResponse {
        limite: 1000,
        saldo: 1000,
    })
}

async fn fetch_account_statement(path: web::Path<(i16,)>) -> impl Responder {
    if path.0 > 5 {
        return HttpResponse::NotFound().finish();
    }

    //todo -> persistence and transaction logic
    HttpResponse::Ok().json(AccountStatementResponse {
        saldo: Balance {
            total: 1000,
            data_extrato: chrono::Local::now().to_rfc3339(),
            limite: 1000,
        },
        ultimas_transacoes: vec![
            TransactionInfo {
                valor: 1000,
                tipo: "c".to_string(),
                descricao: "salario".to_string(),
                realizada_em: chrono::Local::now().to_rfc3339(),
            },
            TransactionInfo {
                valor: 1000,
                tipo: "d".to_string(),
                descricao: "aluguel".to_string(),
                realizada_em: chrono::Local::now().to_rfc3339(),
            },
        ],
    })
}

#[tokio::main]
async fn main() {

    let db_pool = PgPool::connect("postgres://postgres:password@localhost/rinha").await.expect("Can not connect to database");

    let _server = HttpServer::new(move || {
        App::new()
            .route("/clientes/{id}/transacoes", web::post().to(do_transaction))
            .route("/clientes/{id}/extrato", web::get().to(fetch_account_statement))
            .app_data(web::Data::new(db_pool.clone()))
        })
        .bind("localhost:9999").expect("Can not bind to port 9999")
        .run().await.expect("Can not start server");
}
