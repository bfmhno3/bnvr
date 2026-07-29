use std::sync::Arc;

use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::traits::tokio::{Listener as _, Stream as _};
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, ToNsName};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::watch;
use tracing::{error, info};

use super::state::DaemonState;
use crate::{daemon::db, network::bypass, overwrite, profile::crud};

const SOCKET_NAME: &str = "bnvr";

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

struct IpcState {
    daemon: Option<Arc<DaemonState>>,
    shutdown: watch::Sender<bool>,
}

#[derive(Debug, Deserialize)]
struct BypassParams {
    target: String,
}

#[derive(Debug, Deserialize)]
struct SwitchNodeParams {
    node_name: String,
}

#[derive(Debug, Deserialize)]
pub struct TunContext {
    pub device: String,
    pub bypass_routes: Vec<String>,
}

pub async fn listen() -> Result<(), Box<dyn std::error::Error>> {
    let (shutdown, _shutdown_rx) = watch::channel(false);
    listen_on(SOCKET_NAME, None, shutdown).await
}

pub async fn listen_with_state(
    daemon: Arc<DaemonState>,
    shutdown: watch::Sender<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    listen_on(SOCKET_NAME, Some(daemon), shutdown).await
}

pub async fn listen_on(
    name: &str,
    daemon: Option<Arc<DaemonState>>,
    shutdown: watch::Sender<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ns_name = name.to_ns_name::<GenericNamespaced>()?;
    let listener = ListenerOptions::new().name(ns_name).create_tokio()?;
    info!("IPC listening on {name}");

    let state = Arc::new(IpcState { daemon, shutdown });

    loop {
        let stream = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, state).await {
                error!("IPC connection error: {e}");
            }
        });
    }
}

pub async fn send_request(request: &Request) -> Result<Response, Box<dyn std::error::Error>> {
    let ns_name = SOCKET_NAME.to_ns_name::<GenericNamespaced>()?;
    let mut stream = Stream::connect(ns_name).await?;

    let mut msg = serde_json::to_string(request)?;
    msg.push('\n');
    stream.write_all(msg.as_bytes()).await?;

    let (recv_half, _) = stream.split();
    let mut reader = BufReader::new(recv_half);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    Ok(serde_json::from_str(&line)?)
}

pub async fn tun_context() -> Result<Option<TunContext>, Box<dyn std::error::Error>> {
    let request = Request {
        id: 1,
        method: "tun_context".to_string(),
        params: serde_json::Value::Null,
    };
    let response = match send_request(&request).await {
        Ok(response) => response,
        Err(_) => return Ok(None),
    };
    if let Some(error) = response.error {
        return Err(error.into());
    }
    match response.result {
        Some(serde_json::Value::Null) | None => Ok(None),
        Some(value) => serde_json::from_value(value).map(Some).map_err(Into::into),
    }
}

async fn handle_connection(
    stream: Stream,
    state: Arc<IpcState>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (recv_half, send_half) = stream.split();
    let mut reader = BufReader::new(recv_half);
    let mut writer = send_half;
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }

        let resp = match serde_json::from_str::<Request>(&line) {
            Ok(req) => match handle_request(&req, &state).await {
                Ok(result) => Response {
                    id: req.id,
                    result: Some(result),
                    error: None,
                },
                Err(e) => Response {
                    id: req.id,
                    result: None,
                    error: Some(e.to_string()),
                },
            },
            Err(e) => Response {
                id: 0,
                result: None,
                error: Some(format!("invalid request: {e}")),
            },
        };

        let mut msg = serde_json::to_string(&resp)?;
        msg.push('\n');
        writer.write_all(msg.as_bytes()).await?;
    }

    Ok(())
}

async fn handle_request(
    req: &Request,
    state: &IpcState,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match req.method.as_str() {
        "status" => {
            let pid = std::process::id();
            Ok(serde_json::json!({ "status": "running", "pid": pid }))
        }
        "shutdown" => {
            info!("received shutdown request via IPC");
            state.shutdown.send(true)?;
            Ok(serde_json::json!(null))
        }
        "kernel.start" => {
            let daemon = state.daemon.as_ref().ok_or("daemon state not available")?;
            let pid = daemon.kernel.start().await?;
            Ok(serde_json::json!({ "pid": pid }))
        }
        "kernel.stop" => {
            let daemon = state.daemon.as_ref().ok_or("daemon state not available")?;
            daemon.kernel.stop().await?;
            Ok(serde_json::json!(null))
        }
        "kernel.status" => {
            let daemon = state.daemon.as_ref().ok_or("daemon state not available")?;
            let s = daemon.kernel.status().await;
            Ok(serde_json::json!({
                "running": s.running,
                "pid": s.pid,
                "version": s.version,
            }))
        }
        "tun_setup" => {
            let daemon = state.daemon.as_ref().ok_or("daemon state not available")?;
            let device_name = {
                let mut tun = daemon.tun.lock().await;
                tun.setup()?;
                tun.device_name().unwrap_or_default().to_string()
            };
            reload_active_profile(daemon, Some(&device_name)).await?;
            restart_kernel(&daemon.kernel).await?;
            Ok(serde_json::json!({ "device": device_name }))
        }
        "tun_clear" => {
            let daemon = state.daemon.as_ref().ok_or("daemon state not available")?;
            {
                let mut tun = daemon.tun.lock().await;
                tun.clear()?;
            }
            reload_active_profile(daemon, None).await?;
            restart_kernel(&daemon.kernel).await?;
            Ok(serde_json::json!(null))
        }
        "tun_context" => {
            let daemon = state.daemon.as_ref().ok_or("daemon state not available")?;
            let device = {
                let tun = daemon.tun.lock().await;
                tun.device_name().map(str::to_string)
            };
            if let Some(device) = device {
                let conn = db::open()?;
                let bypass_routes = db::list_bypass_routes(&conn)?;
                Ok(serde_json::json!({ "device": device, "bypass_routes": bypass_routes }))
            } else {
                Ok(serde_json::Value::Null)
            }
        }
        "add_bypass" => {
            let daemon = state.daemon.as_ref().ok_or("daemon state not available")?;
            let params: BypassParams = serde_json::from_value(req.params.clone())?;
            let target = bypass::normalize_target(&params.target)?;
            let conn = db::open()?;
            db::add_bypass_route(&conn, &target)?;
            let device = {
                let tun = daemon.tun.lock().await;
                tun.device_name().map(str::to_string)
            };
            reload_active_profile(daemon, device.as_deref()).await?;
            Ok(serde_json::json!({ "target": target }))
        }
        "switch_node" => {
            let params: SwitchNodeParams = serde_json::from_value(req.params.clone())?;
            if let Some(active_plugin) = overwrite::crud::get_active() {
                let config = serde_json::json!({});
                let extra = serde_json::json!({"node_name": params.node_name});
                match overwrite::bridge::run_hook(&active_plugin, "on_node_switch", config, extra)
                    .await
                {
                    Ok(_) => {
                        info!(plugin = %active_plugin, node = %params.node_name, "on_node_switch hook completed")
                    }
                    Err(e) => {
                        error!(plugin = %active_plugin, node = %params.node_name, error = %e, "on_node_switch hook failed")
                    }
                }
            }
            Ok(serde_json::json!({"status": "ok"}))
        }
        _ => Err(format!("unknown method: {}", req.method).into()),
    }
}

async fn reload_active_profile(
    daemon: &DaemonState,
    device_name: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(active) = crud::get_active() else {
        return Ok(());
    };
    let conn = db::open()?;
    let bypass_routes = db::list_bypass_routes(&conn)?;
    crud::materialize_config_with_tun(&active, device_name, &bypass_routes)?;
    let _ = daemon;
    Ok(())
}

async fn restart_kernel(
    kernel: &super::core::KernelManager,
) -> Result<(), Box<dyn std::error::Error>> {
    if kernel.status().await.running {
        kernel.stop().await?;
        let _pid = kernel.start().await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization_roundtrip() {
        let req = Request {
            id: 42,
            method: "status".to_string(),
            params: serde_json::Value::Null,
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, 42);
        assert_eq!(parsed.method, "status");
    }

    #[test]
    fn test_request_deserialize_without_params() {
        let json = r#"{"id":1,"method":"status"}"#;
        let req: Request = serde_json::from_str(json).unwrap();
        assert_eq!(req.id, 1);
        assert_eq!(req.method, "status");
        assert_eq!(req.params, serde_json::Value::Null);
    }

    #[test]
    fn test_request_deserialize_with_params() {
        let json = r#"{"id":2,"method":"query","params":{"target":"example.com"}}"#;
        let req: Request = serde_json::from_str(json).unwrap();
        assert_eq!(req.params["target"], "example.com");
    }

    #[test]
    fn test_response_serialization_with_result() {
        let resp = Response {
            id: 1,
            result: Some(serde_json::json!({"status": "running"})),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_response_serialization_with_error() {
        let resp = Response {
            id: 2,
            result: None,
            error: Some("not found".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"error\""));
        assert!(!json.contains("\"result\""));
    }

    #[test]
    fn test_response_roundtrip() {
        let resp = Response {
            id: 5,
            result: Some(serde_json::json!([1, 2, 3])),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: Response = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, 5);
        assert_eq!(parsed.result.unwrap(), serde_json::json!([1, 2, 3]));
        assert!(parsed.error.is_none());
    }

    #[test]
    fn test_newline_delimited_json_format() {
        let req = Request {
            id: 1,
            method: "ping".to_string(),
            params: serde_json::Value::Null,
        };
        let mut msg = serde_json::to_string(&req).unwrap();
        msg.push('\n');
        assert!(msg.ends_with('\n'));

        let trimmed = msg.trim_end();
        let parsed: Request = serde_json::from_str(trimmed).unwrap();
        assert_eq!(parsed.method, "ping");
    }
}
