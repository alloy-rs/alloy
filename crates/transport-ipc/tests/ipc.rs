//! Drive a real [`IpcConnect`] against an in-process listener.
#![cfg(unix)]
#![allow(missing_docs)]

use alloy_json_rpc::{Id, Request, ResponsePayload};
use alloy_pubsub::PubSubConnect;
use alloy_transport_ipc::IpcConnect;
use interprocess::local_socket::{tokio::prelude::*, GenericFilePath, ListenerOptions, ToFsName};
use serde_json::value::RawValue;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const JSON_RPC_ERROR: &[u8] =
    br#"{"jsonrpc":"2.0","id":1766,"error":{"code":-32000,"message":"filter not found"}}"#;
const NOTIFICATION: &[u8] = br#"{"jsonrpc":"2.0","method":"eth_subscription","params":{"subscription":"0x1","result":"0xabc"}}"#;

fn bind_temp_ipc(
) -> (tempfile::TempDir, std::path::PathBuf, interprocess::local_socket::tokio::Listener) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("alloy.ipc");
    let name = path.as_os_str().to_fs_name::<GenericFilePath>().unwrap();
    let listener = ListenerOptions::new().name(name).create_tokio().unwrap();
    (dir, path, listener)
}

async fn read_n_json_values(reader: &mut (impl AsyncReadExt + Unpin), n: usize) -> Vec<u8> {
    let mut incoming = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        let read = reader.read(&mut buf).await.unwrap();
        assert_ne!(read, 0, "ipc test server got EOF before {n} request(s)");
        incoming.extend_from_slice(&buf[..read]);
        let mut de = serde_json::Deserializer::from_slice(&incoming).into_iter::<Box<RawValue>>();
        let mut seen = 0usize;
        while de.next().is_some_and(|item| item.is_ok()) {
            seen += 1;
        }
        if seen >= n {
            return incoming;
        }
    }
}

/// Concatenated success + JSON-RPC error + subscription notification must all
/// be framed. The error is a protocol response, and the connection stays up
/// for a follow-up request.
#[tokio::test]
async fn ipc_connect_delivers_concatenated_success_error_and_notification() {
    let (_dir, path, listener) = bind_temp_ipc();

    let server = tokio::spawn(async move {
        let stream = listener.accept().await.unwrap();
        let (mut reader, mut writer) = stream.split();

        let _ = read_n_json_values(&mut reader, 2).await;

        let mut body = Vec::new();
        body.extend_from_slice(br#"{"jsonrpc":"2.0","id":1,"result":"0xaaa"}"#);
        body.extend_from_slice(JSON_RPC_ERROR);
        body.extend_from_slice(NOTIFICATION);
        writer.write_all(&body).await.unwrap();
        writer.flush().await.unwrap();

        let _ = read_n_json_values(&mut reader, 1).await;
        writer.write_all(br#"{"jsonrpc":"2.0","id":3,"result":"0xccc"}"#).await.unwrap();
        writer.flush().await.unwrap();
        let _ = reader.read(&mut [0u8; 64]).await;
    });

    let frontend = IpcConnect::new(path).into_service().await.expect("connect ipc");
    let req1 = Request::new("eth_call", Id::Number(1), ()).serialize().unwrap();
    let req2 = Request::new("eth_call", Id::Number(1766), ()).serialize().unwrap();
    let (r1, r2) = tokio::join!(frontend.send(req1), frontend.send(req2));
    let r1 = r1.expect("success response");
    let r2 = r2.expect("json-rpc error is a protocol response");

    assert_eq!(r1.id, Id::Number(1));
    match r1.payload {
        ResponsePayload::Success(_) => {}
        ResponsePayload::Failure(_) => panic!("expected success payload"),
    }
    assert_eq!(r2.id, Id::Number(1766));
    match r2.payload {
        ResponsePayload::Failure(err) => {
            assert_eq!(err.code, -32000);
            assert_eq!(err.message, "filter not found");
        }
        ResponsePayload::Success(_) => panic!("expected error payload"),
    }

    let req3 = Request::new("eth_call", Id::Number(3), ()).serialize().unwrap();
    let r3 = tokio::time::timeout(Duration::from_secs(2), frontend.send(req3))
        .await
        .expect("follow-up timed out")
        .expect("connection survives json-rpc error and notification");
    assert_eq!(r3.id, Id::Number(3));

    drop(frontend);
    let _ = server.await;
}

/// A multi-megabyte result written in small socket chunks must arrive as one
/// response. This is the workload that is quadratic on main.
#[tokio::test]
async fn ipc_connect_reads_chunked_multimegabyte_response() {
    let (_dir, path, listener) = bind_temp_ipc();
    const PAYLOAD: usize = 2 * 1024 * 1024;
    const CHUNK: usize = 4096;

    let server = tokio::spawn(async move {
        let stream = listener.accept().await.unwrap();
        let (mut reader, mut writer) = stream.split();
        let _ = read_n_json_values(&mut reader, 1).await;

        let mut body = Vec::from(br#"{"jsonrpc":"2.0","id":1,"result":"0x"#);
        body.extend(vec![b'0'; PAYLOAD]);
        body.extend_from_slice(br#""}"#);

        for chunk in body.chunks(CHUNK) {
            writer.write_all(chunk).await.unwrap();
        }
        writer.flush().await.unwrap();
        let _ = reader.read(&mut [0u8; 64]).await;
    });

    let frontend = IpcConnect::new(path).into_service().await.expect("connect ipc");
    let req = Request::new("eth_call", Id::Number(1), ()).serialize().unwrap();
    let resp = tokio::time::timeout(Duration::from_secs(10), frontend.send(req))
        .await
        .expect("large response timed out")
        .expect("large response");
    assert_eq!(resp.id, Id::Number(1));
    match resp.payload {
        ResponsePayload::Success(raw) => {
            assert!(raw.get().len() > PAYLOAD, "result should carry the multi-MB payload");
        }
        ResponsePayload::Failure(err) => panic!("unexpected error: {err:?}"),
    }

    drop(frontend);
    let _ = server.await;
}
