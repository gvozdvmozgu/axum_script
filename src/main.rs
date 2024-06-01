use deno_core::op2;
use deno_core::OpState;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::rc::Rc;

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
deno_core::extension!(my_extension, ops = [op_route]);

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

#[tokio::main]
async fn main() {
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

    let mut route_map: HashMap<String, v8::Global<v8::Function>> = HashMap::new();

    let hmref = Rc::new(RefCell::new(route_map));
    js_runtime.op_state().borrow_mut().put(Rc::clone(&hmref));
    let mod_id = js_runtime.load_main_es_module(&init_module).await;
    let result = js_runtime.mod_evaluate(mod_id.unwrap());
    js_runtime.run_event_loop(Default::default()).await.unwrap();
    dbg!(Rc::clone(&hmref));
    result.await.unwrap();
    let hm = hmref.borrow();
    // https://stackoverflow.com/a/76376307/19839414

    let gf = hm.get("foo").unwrap().clone();
    let nnf = gf.open(js_runtime.v8_isolate());
    let scope = &mut js_runtime.handle_scope();
    let global = js_runtime.get_module_namespace(mod_id.unwrap()).unwrap();
    //let global_value: v8::Global<v8::Value> = Cast(global);
    //let newval: v8::Value = v8::Object::new(scope).into();
    //let global_obj = global.open(js_runtime.v8_isolate());
    //let global_val: v8::Value = global_obj.into();
    //let recv: v8::Local<v8::Value> = v8::Local::new(scope, global_obj);
    let recv: v8::Local<v8::Value> = global.into();
    let func_res = nnf.call(scope, recv, &[]);

    /*match res {
        Ok(_) => (),
        Err(s) => {dbg!()}
    }*/
}
