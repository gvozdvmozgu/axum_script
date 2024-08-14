((globalThis) => {
  const core = Deno.core;

  function argsToMessage(...args) {
    return args
      .map((arg) => (typeof arg === "string" ? arg : JSON.stringify(arg)))
      .join(" ");
  }

  globalThis.console = {
    log: (...args) => {
      core.print(`${argsToMessage(...args)}\n`, false);
    },
    error: (...args) => {
      core.print(`[err]: ${argsToMessage(...args)}\n`, true);
    },
  };

  globalThis.route = Deno.core.ops.op_route;
  globalThis.query = Deno.core.ops.op_query;
  globalThis.execute = Deno.core.ops.op_execute;
  globalThis.sleep = Deno.core.ops.op_sleep;
  globalThis.createCache = Deno.core.ops.op_create_cache;
  globalThis.flushCache = Deno.core.ops.op_flush_cache;
  globalThis.connectToDatabase = Deno.core.ops.op_connect_db;
  globalThis.getCache = (subset) =>
    typeof subset === "function"
      ? Deno.core.ops.op_with_cache(subset)
      : subset
      ? Deno.core.ops.op_get_cache_subset_value(subset)
      : Deno.core.ops.op_get_cache_value();
})(globalThis);
