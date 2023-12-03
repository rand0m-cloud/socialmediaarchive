use std::{
    collections::HashMap,
    io::Write,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use crate::LocalClient;
use actix_web::{web, *};
use anyhow::Result;
use futures::{future::abortable, stream::AbortHandle, Future, FutureExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, to_value, Value};
use tokio::sync::Mutex;
use tracing::error;

/// Global incrementing counter for task IDs
pub static TASK_ID: AtomicU32 = AtomicU32::new(0);

/// The global data used in the daemon. Clones are referenced counted.
#[derive(Clone)]
pub struct Daemon {
    client: LocalClient,
    tasks: Arc<Mutex<HashMap<u32, Task>>>,
}

/// An abortable task created from a future that results in a json value
#[derive(Serialize, Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Task {
    InProgress {
        #[serde(skip)]
        abort_handle: Option<AbortHandle>,
    },
    Cancelled,
    Completed {
        data: Value,
    },
}

impl Daemon {
    pub fn new(client: LocalClient) -> Self {
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

        self.tasks.lock().await.insert(
            id,
            Task::InProgress {
                abort_handle: Some(abort_handle),
            },
        );

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
                abort_handle.as_ref().unwrap().abort();
                *t = Task::Cancelled;
            }
        }
    }

    /// Query for the task.
    pub async fn get_task(&self, id: u32) -> Option<Task> {
        self.tasks.lock().await.get(&id).cloned()
    }
}

/// Starts a daemon from the given `Client`
pub async fn run(client: LocalClient) -> Result<()> {
    let daemon = Daemon::new(client);
    HttpServer::new(move || {
        use endpoints::*;

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

/// The input data for the add link endpoint
#[derive(Deserialize, Serialize, Clone)]
pub struct AddLink {
    pub link: String,
    pub description: String,
}

/// The API endpoints
pub mod endpoints {
    use actix_web::{http::Method, web, *};
    use serde_json::json;

    use super::{to_responder, AddLink};
    use crate::{api::ClientApi, daemon::Daemon};

    #[post("/search")]
    async fn search_endpoint(
        name: String,
        daemon: web::Data<Daemon>,
        req: HttpRequest,
    ) -> impl Responder {
        to_responder(&daemon, req, name, |daemon, name| async move {
            daemon.client.search(&name).await
        })
        .await
    }

    #[post("/add")]
    async fn add_endpoint(
        input: web::Json<AddLink>,
        daemon: web::Data<Daemon>,
        req: HttpRequest,
    ) -> impl Responder {
        to_responder(
            &daemon,
            req,
            input.into_inner(),
            |daemon, input| async move {
                daemon
                    .client
                    .add_link(&input.link, &input.description)
                    .await
            },
        )
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
}

/// Transforms a future into a task and responds with a 202 Accepted that contains
/// a Location header for the query task endpoint. Maps the future's result into the
/// data or a serialized error.
///
/// If the task fails, the input data is saved in a ndjson file `failed_tasks.ndjson` for possible retries.
async fn to_responder<
    In: Serialize + Clone + 'static,
    Out: Serialize,
    F: FnOnce(Daemon, In) -> Fut,
    Fut: Future<Output = Result<Out>> + 'static,
>(
    daemon: &Daemon,
    req: HttpRequest,
    input: In,
    res: F,
) -> impl Responder {
    let task_id = daemon
        .new_task(
            res(daemon.clone(), input.clone()).map(move |res| match res {
                Ok(t) => to_value(t).unwrap(),
                Err(e) => {
                    error!("daemon error: {e}");
                    let err = json!({
                        "error": e.to_string(),
                        "backtrace": e.chain().map(|err| err.to_string()).collect::<Vec<_>>(),
                        "input": to_value(input).unwrap(),
                        "path": req.path()
                    });

                    // save the failed task for possible replay
                    let mut failed_tasks_file = std::fs::File::options()
                        .append(true)
                        .create(true)
                        .open("failed_tasks.ndjson")
                        .unwrap();
                    writeln!(
                        &mut failed_tasks_file,
                        "{}",
                        serde_json::to_string(&err).unwrap()
                    )
                    .unwrap();

                    err
                }
            }),
        )
        .await;

    HttpResponse::Accepted()
        .insert_header(("location", format!("/api/v0/task/{task_id}")))
        .finish()
}
