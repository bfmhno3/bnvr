use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, ToNsName};
use interprocess::local_socket::traits::tokio::{Listener as _, Stream as _};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{error, info};

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

pub async fn listen() -> Result<(), Box<dyn std::error::Error>> {
    listen_on(SOCKET_NAME).await
}

pub async fn listen_on(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ns_name = name.to_ns_name::<GenericNamespaced>()?;
    let listener = ListenerOptions::new().name(ns_name).create_tokio()?;
    info!("IPC listening on {name}");

    loop {
        let stream = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream).await {
                error!("IPC connection error: {e}");
            }
        });
    }
}

async fn handle_connection(stream: Stream) -> Result<(), Box<dyn std::error::Error>> {
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
            Ok(req) => match handle_request(&req).await {
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
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match req.method.as_str() {
        "status" => {
            let pid = std::process::id();
            Ok(serde_json::json!({ "status": "running", "pid": pid }))
        }
        "shutdown" => {
            info!("received shutdown request via IPC");
            trigger_shutdown();
            Ok(serde_json::json!(null))
        }
        _ => Err(format!("unknown method: {}", req.method).into()),
    }
}

fn trigger_shutdown() {
    #[cfg(unix)]
    {
        // SAFETY: Sending SIGINT to our own process to trigger graceful shutdown
        unsafe {
            libc::kill(libc::getpid(), libc::SIGINT);
        }
    }
    #[cfg(windows)]
    {
        std::process::exit(0);
    }
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

        // Should parse after trimming or with trailing newline
        let trimmed = msg.trim_end();
        let parsed: Request = serde_json::from_str(trimmed).unwrap();
        assert_eq!(parsed.method, "ping");
    }
}

