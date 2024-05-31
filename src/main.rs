use deno_core::error::AnyError;
use std::env;
use std::rc::Rc;

fn get_init_dir() -> String {
    let args: Vec<String> = env::args().collect();
    return if (args.len() < 2) {
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
        ..Default::default()
    });
    let mod_id = js_runtime.load_main_es_module(&init_module).await;
    let result = js_runtime.mod_evaluate(mod_id.unwrap());
    js_runtime.run_event_loop(Default::default()).await;
    result.await;
}
