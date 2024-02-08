use std::sync::Arc;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chrono;

#[derive(serde::Deserialize, Debug)]
struct Transaction {
    tipo: String,
    valor: u64,
    descricao: String,
}

#[derive(serde::Serialize)]
struct TransactionResponse {
    limite: i64,
    saldo: i64,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AccountStatementResponse {
    pub saldo: Balance,
    pub ultimas_transacoes: Vec<LastTransactions>,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Balance {
    pub total: i64,
    pub data_extrato: String,
    pub limite: i64,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LastTransactions {
    pub valor: i64,
    pub tipo: String,
    pub descricao: String,
    pub realizada_em: String,
}


async fn do_transaction(path: web::Path<(u16,)>, data: web::Json<Transaction>) -> impl Responder {
    println!("do transaction called with data {:?} and client_id {:?}", data, path.0);
    if path.0 > 6 {
        return HttpResponse::NotFound().json("client not found");
    }

    if data.tipo != "c" && data.tipo != "d" {
        return HttpResponse::BadRequest().json("type must be 'c' or 'd'");
    }

    if data.descricao.len() < 1 || data.descricao.len() > 10 {
        return HttpResponse::BadRequest().json("description must be between 1 and 10 characters long");
    }

    //todo -> persistence and transaction logic

    HttpResponse::Ok().json(TransactionResponse {
        limite: 1000,
        saldo: 1000,
    })
}

async fn fetch_account_statement(path: web::Path<(u16,)>) -> impl Responder {
    println!("fetch account statement called");
    if path.0 > 6 {
        return HttpResponse::NotFound().json("client not found");
    }

    //todo -> persistence and transaction logic
    HttpResponse::Ok().json(AccountStatementResponse {
        saldo: Balance {
            total: 1000,
            data_extrato: chrono::Local::now().to_rfc3339(),
            limite: 1000,
        },
        ultimas_transacoes: vec![
            LastTransactions {
                valor: 1000,
                tipo: "c".to_string(),
                descricao: "salario".to_string(),
                realizada_em: chrono::Local::now().to_rfc3339(),
            },
            LastTransactions {
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
    let _server = HttpServer::new(|| {
        App::new()
            .route("/clientes/{id}/transacoes", web::post().to(do_transaction))
            .route("/clientes/{id}/extrato", web::get().to(fetch_account_statement))
        })
        .bind("localhost:9999").expect("Can not bind to port 9999")
        .run().await.unwrap();
}
