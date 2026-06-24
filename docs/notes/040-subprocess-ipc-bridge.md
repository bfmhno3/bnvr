# 040 - Subprocess IPC Bridge (Rust to Python)

## Pattern

Spawning a child process and communicating via stdin/stdout JSON is a simple,
portable IPC mechanism. The parent writes a JSON request to the child's stdin,
closes stdin (sending EOF), then reads a JSON response from stdout.

```rust
let mut child = Command::new(&python)
    .arg(&script)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()?;

if let Some(mut stdin) = child.stdin.take() {
    stdin.write_all(json_bytes).await?;
    stdin.shutdown().await?;  // sends EOF
}

let output = child.wait_with_output().await?;
let response: Response = serde_json::from_slice(&output.stdout)?;
```

## Key Points

- `wait_with_output()` takes `self` by value, consuming the `Child`. Save
  `child.id()` before calling it if you need to kill the process later.
- `stdin.shutdown().await` is critical -- the child blocks on `json.load(sys.stdin)`
  until it sees EOF. Without shutdown, the child hangs forever.
- Use `tokio::time::timeout` to enforce a deadline. On timeout, kill the child
  by PID since the `Child` was consumed by `wait_with_output`.
- On Windows, `taskkill /PID <pid> /F /T` kills a process tree. On Unix,
  `kill(pid, SIGKILL)` works.

## Test Isolation Pitfall

Tests that mutate process-global state (like environment variables) must not
leak into other tests running in parallel. The Rust test harness runs tests
from the same binary in parallel by default.

**Wrong:** Set `BNVR_HOME` env var in test, rely on other tests not reading it.

**Right:** Refactor functions to accept explicit parameters (`_in(dir)` variants)
so tests pass their own temp directories without touching env vars.

```rust
// Production API uses default path
pub fn list() -> Result<Vec<Plugin>> { list_in(&paths::overwrite_dir()) }

// Testable variant accepts explicit directory
pub fn list_in(dir: &Path) -> Result<Vec<Plugin>> { ... }

#[test]
fn test_list() {
    let tmp = setup("list-test");
    let plugins = list_in(&tmp).unwrap();
    // ...
    let _ = fs::remove_dir_all(tmp);
}
```

## Python Template Pattern

Embedding a Python script as a `const &str` in Rust keeps the template
bundled with the binary. The template uses a dispatch table for hooks:

```python
handlers = {
    "preprocess": preprocess,
    "postprocess": postprocess,
    "on_node_switch": lambda c: on_node_switch(c, extra.get("node_name", "")),
    "on_network_dropped": on_network_dropped,
}
result = handlers[hook](config)
```

This makes it easy for users to add custom logic while maintaining a
stable IPC contract.
