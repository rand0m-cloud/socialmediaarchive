use crate::client::*;
use actix_web::{web, *};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;

pub async fn run(client: Client) -> Result<()> {
    HttpServer::new(move || {
        App::new()
            .service(search_endpoint)
            .service(add_endpoint)
            .app_data(web::Data::new(client.clone()))
    })
    .bind(("0.0.0.0", 5003))?
    .run()
    .await?;
    Ok(())
}

#[post("/search")]
async fn search_endpoint(name: String, client: web::Data<Client>) -> impl Responder {
    to_responder(client.search(&name).await)
}

#[derive(Deserialize)]
struct AddLink {
    link: String,
    description: String,
}

#[post("/add")]
async fn add_endpoint(input: web::Json<AddLink>, client: web::Data<Client>) -> impl Responder {
    to_responder(client.add_link(&input.link, &input.description).await)
}

fn to_responder<T: Serialize>(res: Result<T>) -> impl Responder {
    match res {
        Ok(t) => HttpResponse::Ok().json(t),
        Err(e) => {
            error!("daemon error: {e}");
            HttpResponse::InternalServerError().json(
                json!({
                    "error": e.to_string(),
                    "backtrace": e.chain().map(|err| err.to_string()).collect::<Vec<_>>()
                })
            )
        }
    }
}
