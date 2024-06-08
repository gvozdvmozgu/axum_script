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
use deno_core::op2;
use deno_core::JsRuntime;
use deno_core::OpState;
use serde_json::{json, Value};
use sqltojson::row_to_json;
use sqlx::Pool;
use sqlx::{migrate::MigrateDatabase, Any, AnyPool, Sqlite};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::rc::Rc;
use std::thread;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

mod sqltojson;

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

deno_core::extension!(
    my_extension,
    ops = [op_route, op_query],
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

#[derive(Clone)]
struct JsRunner {
    routes: Rc<RefCell<HashMap<String, v8::Global<v8::Function>>>>,
    runtime: Rc<RefCell<JsRuntime>>,
    // db_pool: Pool<Sqlite>,
}

impl JsRunner {
    async fn new() -> JsRunner {
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

        return JsRunner {
            routes: Rc::clone(&hmref),
            runtime: Rc::new(RefCell::new(js_runtime)),
            // db_pool: pool,
        };
    }

    async fn run_loop(&self, mut rx_req: mpsc::Receiver<RouteRequest>) {
        while let Some(req) = rx_req.recv().await {
            let response = self.run_route(&req.route_name).await;
            req.response_channel.send(response).unwrap();
        }
    }

    #[tokio::main(flavor = "current_thread")]
    async fn run_thread(rx_req: mpsc::Receiver<RouteRequest>) {
        let runner = JsRunner::new().await;
        runner.run_loop(rx_req).await;
    }

    fn spawn_thread() -> mpsc::Sender<RouteRequest> {
        let (tx_req, rx_req) = mpsc::channel(32);
        thread::spawn(|| {
            JsRunner::run_thread(rx_req);
        });
        return tx_req;
    }
    async fn run_route(&self, route_name: &str) -> Response<Body> {
        dbg!(route_name);
        let hm = self.routes.borrow();
        let mut runtime = self.runtime.borrow_mut();
        //let tgf = hm.get(route_name).unwrap();
        if let Some(gf) = hm.get(route_name) {
            let func_res_promise = runtime.call(gf); //.await.unwrap();
            let func_res0 = runtime
                .with_event_loop_promise(func_res_promise, Default::default())
                .await
                .unwrap();

            //let func_res0 = func_res_promise.await.unwrap();
            let scope = &mut runtime.handle_scope();
            let func_res = func_res0.open(scope);

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

struct RouteRequest {
    route_name: String,
    response_channel: oneshot::Sender<Response<Body>>,
    request: Request,
}

#[derive(Clone)]
struct RouteState {
    tx_req: mpsc::Sender<RouteRequest>,
}

#[tokio::main]
async fn main() {
    let tx_req = JsRunner::spawn_thread();
    //.join()
    //.expect("Thread panicked");
    print!("Starting server");
    let rstate = RouteState { tx_req: tx_req };
    let app = Router::new()
        .route("/", get(req_handler))
        .route("/*key", get(req_handler))
        .with_state(rstate);
    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:4000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn req_handler(
    State(state): State<RouteState>,
    match_path: MatchedPath,
    raw_params: RawPathParams,
    req: Request,
) -> Response<Body> {
    let path = req.uri().path();
    dbg!(path);
    dbg!(match_path);
    dbg!(raw_params);
    let (tx, rx) = oneshot::channel();
    state
        .tx_req
        .send(RouteRequest {
            route_name: String::from(path),
            response_channel: tx,
            request: req,
        })
        .await
        .unwrap();
    match rx.await {
        Ok(v) => v,
        Err(e) => {
            dbg!(e);
            panic!("the sender dropped")
        }
    }
}
