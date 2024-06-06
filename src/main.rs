use axum::body::Body;
use axum::response::{IntoResponse, Response};
use axum::{extract::State, response::Html, routing::get, routing::post, Json, Router};
use deno_core::op2;
use deno_core::JsRuntime;
use deno_core::OpState;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::rc::Rc;
use std::thread;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
/*const ROUTES: OnceCell<HashMap<String, v8::Global<v8::Function>>> = OnceCell::new();

fn routes_map() -> &'static Mutex<HashMap<String, v8::Global<v8::Function>>> {
    static ARRAY: OnceLock<Mutex<Vec<u8>>> = OnceLock::new();
    ARRAY.get_or_init(|| Mutex::new(vec![]))
}
*/
#[op2()]
fn op_route(state: &mut OpState, #[string] path: &str, #[global] router: v8::Global<v8::Function>) {
    let hmref = state.borrow::<Rc<RefCell<HashMap<String, v8::Global<v8::Function>>>>>();
    let mut routes = hmref.borrow_mut();
    routes.insert(String::from(path), router);
    //routes.set(*current_routes);
    dbg!(path);
    ()
}
deno_core::extension!(my_extension, ops = [op_route], js = ["src/runtime.js"]);

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
#[derive(Clone)]
struct AppState {
    routes: Rc<RefCell<HashMap<String, v8::Global<v8::Function>>>>,
    runtime: Rc<RefCell<JsRuntime>>,
}

struct Request {
    route_name: String,
    response_channel: oneshot::Sender<Response<Body>>,
}

#[derive(Clone)]
struct RouteState {
    tx_req: mpsc::Sender<Request>,
}
#[tokio::main]
async fn js_thread(mut rx_req: mpsc::Receiver<Request>) {
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

    let route_map: HashMap<String, v8::Global<v8::Function>> = HashMap::new();

    let hmref = Rc::new(RefCell::new(route_map));
    js_runtime.op_state().borrow_mut().put(Rc::clone(&hmref));
    let mod_id = js_runtime.load_main_es_module(&init_module).await;
    let result = js_runtime.mod_evaluate(mod_id.unwrap());
    js_runtime.run_event_loop(Default::default()).await.unwrap();
    result.await.unwrap();

    let state = AppState {
        routes: Rc::clone(&hmref),
        runtime: Rc::new(RefCell::new(js_runtime)),
    };
    //run_route(state, "foo").await;
    while let Some(req) = rx_req.recv().await {
        println!("GOT = {}", req.route_name);
        req.response_channel
            .send(run_route(&state, "foo").await)
            .unwrap();
    }
}

#[tokio::main]
async fn main() {
    let (tx_req, rx_req) = mpsc::channel(32);
    thread::spawn(|| {
        js_thread(rx_req);
    });
    //.join()
    //.expect("Thread panicked");
    print!("Starting server");
    let rstate = RouteState { tx_req: tx_req };
    let app = Router::new()
        .route("/", get(route_handler))
        .with_state(rstate);
    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:4000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn route_handler(State(state): State<RouteState>) -> Response<Body> {
    let (tx, rx) = oneshot::channel();
    state
        .tx_req
        .send(Request {
            route_name: String::from("foo"),
            response_channel: tx,
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

async fn run_route(state: &AppState, route_name: &str) -> Response<Body> {
    let hm = state.routes.borrow();
    let mut runtime = state.runtime.borrow_mut();
    let gf = hm.get(route_name).unwrap();
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
        print!("{}", s);
        return Html(s).into_response();
    } else {
        return Html("").into_response();
    }
}
