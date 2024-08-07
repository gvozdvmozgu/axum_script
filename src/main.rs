use axum::body::Body;
use axum::extract::{MatchedPath, RawPathParams};
use axum::response::{IntoResponse, Response};
use axum::{
    extract::{Request, State},
    http::StatusCode,
    response::Html,
    routing::get,
    Router,
};
use deno_core::error::AnyError;
use deno_core::op2;
use deno_core::JsRuntime;
use deno_core::{serde_v8::to_v8, OpState};
use futures::channel::{mpsc, oneshot};

use futures::{SinkExt, StreamExt};
use serde_json::Value;
use sqltojson::row_to_json;
use sqlx::Pool;
use sqlx::{migrate::MigrateDatabase, Any, AnyPool, Sqlite};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use tokio::time::{sleep, Duration};

mod sqltojson;

lazy_static::lazy_static! {
    pub static ref TOKIO_RUNTIME: tokio::runtime::Runtime =
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
}

struct JsRunnerInner {
    runtime: JsRuntime,
    routes: HashMap<String, v8::Global<v8::Function>>,
}

impl JsRunner {
    pub async fn eval(&mut self, route_request: RouteRequest) -> Response<Body> {
        let (resp_tx, resp_rx) = oneshot::channel::<Response<Body>>();
        let cmd = Command::Eval {
            responder: resp_tx,
            route_request,
        };

        // Send request
        match self.sender.send(cmd).await {
            Ok(_) => {
                // All good
            }
            Err(err) => todo!("{err:?}"),
        }

        // Wait for result
        match resp_rx.await {
            Ok(t) => t,
            Err(err) => todo!("{err:?}"),
        }
    }
}

impl JsRunnerInner {
    async fn new() -> JsRunnerInner {
        let dir = get_init_dir();
        let setup_path = [dir, String::from("setup.js")].concat();

        let init_module =
            deno_core::resolve_path(&setup_path, env::current_dir().unwrap().as_path()).unwrap();
        let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
            module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
            extensions: vec![my_extension::init_ops_and_esm()],
            ..Default::default()
        });
        // following https://github.com/DataDog/datadog-static-analyzer/blob/cde26f42f1cdbbeb09650403318234f277138bbd/crates/static-analysis-kernel/src/analysis/ddsa_lib/runtime.rs#L54
        let pool = Rc::new(RefCell::new(connect_database("sqlite://sqlite.db").await));

        let route_map: HashMap<String, v8::Global<v8::Function>> = HashMap::new();

        let hmref = Rc::new(RefCell::new(route_map));
        js_runtime.op_state().borrow_mut().put(Rc::clone(&pool));
        js_runtime.op_state().borrow_mut().put(Rc::clone(&hmref));
        let mod_id = js_runtime.load_main_es_module(&init_module).await;
        let result = js_runtime.mod_evaluate(mod_id.unwrap());
        js_runtime.run_event_loop(Default::default()).await.unwrap();
        result.await.unwrap();

        return JsRunnerInner {
            routes: (*hmref.borrow()).clone(),
            runtime: js_runtime,
            // db_pool: pool,
        };
    }

    fn to_call_args(runtime: &mut JsRuntime, req: &RouteRequest) -> v8::Global<v8::Value> {
        let mut scope = &mut runtime.handle_scope();
        let v8_arg: v8::Local<v8::Value> = to_v8(
            &mut scope,
            serde_json::Value::Object(req.route_args.clone()),
        )
        .unwrap();
        return v8::Global::new(&mut *scope, v8_arg);
    }

    async fn run_route(&mut self, req: &RouteRequest) -> Response<Body> {
        if let Some(caller) = self.routes.get(&*(req.route_name)) {
            let runtime = &mut self.runtime;
            let args = vec![Self::to_call_args(runtime, req)];

            let func_res_promise = runtime.call_with_args(caller, &args);
            let func_res0 = runtime
                .with_event_loop_promise(func_res_promise, Default::default())
                .await;
            if let Err(e) = func_res0 {
                dbg!(e);
                return (StatusCode::INTERNAL_SERVER_ERROR, Html("Error")).into_response();
            }
            let func_res1 = func_res0.unwrap();
            let scope = &mut runtime.handle_scope();

            //let func_res0 = func_res_promise.await.unwrap();
            let func_res = func_res1.open(scope);

            if func_res.is_string() {
                let s = func_res
                    .to_string(scope)
                    .unwrap()
                    .to_rust_string_lossy(scope);
                return Html(s).into_response();
            } else {
                return Html("").into_response();
            }
        } else {
            return (StatusCode::NOT_FOUND, Html("404 not found")).into_response();
        }
    }
}

enum Command {
    Eval {
        route_request: RouteRequest,
        responder: oneshot::Sender<Response<Body>>,
    },
}

#[derive(Clone)]
struct JsRunner {
    _handle: Arc<JoinHandle<Result<(), AnyError>>>,
    sender: mpsc::Sender<Command>,
}

impl JsRunner {
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::channel::<Command>(32);

        let handle = Arc::new(thread::spawn(move || {
            TOKIO_RUNTIME.block_on(async {
                let mut inner = JsRunnerInner::new().await;
                while let Some(cmd) = receiver.next().await {
                    match cmd {
                        Command::Eval {
                            responder,
                            route_request,
                        } => {
                            let t = inner.run_route(&route_request).await;
                            responder.send(t).unwrap();
                        }
                    }
                }
                Ok::<(), AnyError>(())
            })?;

            Ok(())
        }));

        Self {
            _handle: handle,
            sender,
        }
    }
}

#[op2()]
fn op_route(state: &mut OpState, #[string] path: &str, #[global] router: v8::Global<v8::Function>) {
    let hmref = state.borrow::<Rc<RefCell<HashMap<String, v8::Global<v8::Function>>>>>();
    let mut routes = hmref.borrow_mut();
    routes.insert(String::from(path), router);
    ()
}

#[op2(async)]
#[serde]
async fn op_query(state: Rc<RefCell<OpState>>, #[string] sqlq: String) -> serde_json::Value {
    let state = state.borrow();
    let poolref = state.borrow::<Rc<RefCell<Pool<Any>>>>();
    let pool = poolref.borrow();
    let rows = sqlx::query(&sqlq).fetch_all(&(*pool)).await.unwrap();
    let rows: Vec<Value> = rows.iter().map(row_to_json).collect();
    //dbg!(&rows);
    return Value::Array(rows);
    //    return rows.len().try_into().unwrap();
}

#[op2(async)]
async fn op_sleep(ms: u32) {
    sleep(Duration::from_millis(ms.into())).await;
}

deno_core::extension!(
    my_extension,
    ops = [op_route, op_query, op_sleep],
    js = ["src/runtime.js"]
);

fn get_init_dir() -> String {
    let args: Vec<String> = env::args().collect();
    return if args.len() < 2 {
        env::current_dir()
            .unwrap()
            .into_os_string()
            .into_string()
            .unwrap()
    } else {
        args[1].clone()
    };
}

async fn connect_database(db_url: &str) -> Pool<Any> {
    sqlx::any::install_default_drivers();
    if !Sqlite::database_exists(db_url).await.unwrap_or(false) {
        println!("Creating database {}", db_url);
        match Sqlite::create_database(db_url).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        println!("Database already exists");
    }
    let db = AnyPool::connect(db_url).await.unwrap();
    return db;
}

struct RouteRequest {
    route_name: String,
    route_args: serde_json::Map<String, Value>,
    request: Request,
}

#[derive(Clone)]
struct RouteState {
    runner: JsRunner,
}

#[tokio::main]
async fn main() {
    print!("Starting server");

    // FIXME:
    let t = JsRunnerInner::new().await;
    let paths = t.routes.keys();

    let rstate = RouteState {
        runner: JsRunner::new(),
    };
    let app: Router = paths
        .fold(Router::new(), |router, path| {
            router.route(path, get(req_handler))
        })
        .with_state(rstate);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:4000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn req_handler(
    State(mut state): State<RouteState>,
    match_path: MatchedPath,
    raw_params: RawPathParams,
    req: Request,
) -> Response<Body> {
    let path = match_path.as_str();
    let parvals =
        serde_json::Map::from_iter(raw_params.iter().map(|(k, v)| (String::from(k), v.into())));

    let t = state
        .runner
        .eval(RouteRequest {
            route_name: String::from(path),
            route_args: parvals,
            request: req,
        })
        .await;

    t
}
