use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use crate::client::*;
use actix_web::{http::Method, web, *};
use anyhow::Result;
use futures::{future::abortable, stream::AbortHandle, Future, FutureExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, to_value, Value};
use tokio::sync::Mutex;
use tracing::error;

/// Global incrementing counter for task IDs
pub static TASK_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Clone)]
pub struct Daemon {
    client: Client,
    tasks: Arc<Mutex<HashMap<u32, Task>>>,
}

/// An abortable task created from a future that results in a Value
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Task {
    InProgress {
        #[serde(skip)]
        abort_handle: AbortHandle,
    },
    Completed {
        data: Value,
    },
    Cancelled,
}

impl Daemon {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            tasks: Default::default(),
        }
    }

    /// Adds a new task and spawns the future, returns the ID to query the task with.
    pub async fn new_task<F: Future<Output = Value> + 'static>(&self, f: F) -> u32 {
        let id = TASK_ID.fetch_add(1, Ordering::Relaxed);
        let (fut, abort_handle) = {
            let this = self.clone();
            abortable(f.then(move |v| async move { this.set_task_completed(id, v).await }))
        };

        tokio::task::spawn_local(fut);

        self.tasks
            .lock()
            .await
            .insert(id, Task::InProgress { abort_handle });

        id
    }

    /// Marks the task complete with its resulting data. Ignores if the task was cancelled.
    pub async fn set_task_completed(&self, id: u32, data: Value) {
        let mut tasks = self.tasks.lock().await;

        if tasks
            .get(&id)
            .map(|t| !matches!(t, Task::Cancelled))
            .unwrap_or(true)
        {
            tasks.insert(id, Task::Completed { data });
        }
    }

    /// Cancels the task by aborting the future, ignores if task isn't in progress.
    pub async fn cancel_task(&self, id: u32) {
        if let Some(t) = self.tasks.lock().await.get_mut(&id) {
            if let Task::InProgress { abort_handle } = t {
                abort_handle.abort();
                *t = Task::Cancelled;
            }
        }
    }

    /// Query for the task.
    pub async fn get_task(&self, id: u32) -> Option<Task> {
        self.tasks.lock().await.get(&id).cloned()
    }
}

pub async fn run(client: Client) -> Result<()> {
    let daemon = Daemon::new(client);
    HttpServer::new(move || {
        App::new()
            .service(
                web::scope("/api/v0")
                    .service(search_endpoint)
                    .service(add_endpoint)
                    .service(task_endpoint),
            )
            .app_data(web::Data::new(daemon.clone()))
    })
    .bind(("0.0.0.0", 5003))?
    .run()
    .await?;
    Ok(())
}

#[post("/search")]
async fn search_endpoint(name: String, daemon: web::Data<Daemon>) -> impl Responder {
    to_responder(daemon.get_ref().clone(), async move {
        daemon.client.search(&name).await
    })
    .await
}

#[derive(Deserialize)]
struct AddLink {
    link: String,
    description: String,
}

#[post("/add")]
async fn add_endpoint(input: web::Json<AddLink>, daemon: web::Data<Daemon>) -> impl Responder {
    to_responder(daemon.get_ref().clone(), async move {
        daemon
            .client
            .add_link(&input.link, &input.description)
            .await
    })
    .await
}

#[route("/task/{task_id}", method = "GET", method = "DELETE")]
async fn task_endpoint(
    task_id: web::Path<u32>,
    method: Method,
    daemon: web::Data<Daemon>,
) -> impl Responder {
    match method {
        _ if method == Method::GET => match daemon.get_task(*task_id).await {
            Some(t) => HttpResponse::Ok().json(t),
            None => HttpResponse::BadRequest().json(json!({"error": "unknown task"})),
        },
        _ if method == Method::DELETE => {
            daemon.cancel_task(*task_id).await;
            HttpResponse::Ok().finish()
        }
        _ => HttpResponse::BadRequest().finish(),
    }
}

/// Transforms a future into a task and responds with a 202 Accepted that contains
/// a Location header for the query endpoint. Maps the future's result into the
/// data or a serialized error.
async fn to_responder<T: Serialize, F: Future<Output = Result<T>> + 'static>(
    daemon: Daemon,
    res: F,
) -> impl Responder {
    let task_id = daemon
        .new_task(res.map(|res| match res {
            Ok(t) => to_value(t).unwrap(),
            Err(e) => {
                error!("daemon error: {e}");
                json!({
                    "error": e.to_string(),
                    "backtrace": e.chain().map(|err| err.to_string()).collect::<Vec<_>>()
                })
            }
        }))
        .await;

    HttpResponse::Accepted()
        .insert_header(("location", format!("/api/v0/task/{task_id}")))
        .finish()
}
