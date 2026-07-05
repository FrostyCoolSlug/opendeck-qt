//! Simple event loop, invoke_method_with_args is thread safe, so we can keep it safe here.

use qtbridge::QmlMethodInvoker;
use qtbridge::qtbridge_type_lib::{QList, QVariant};
use serde_json::Value;
use std::sync::OnceLock;
use serde::Serialize;
use tokio::sync::mpsc;

static EVENT_TX: OnceLock<mpsc::UnboundedSender<(String, Value)>> = OnceLock::new();

pub fn init_event_system(invoker: QmlMethodInvoker) {
    let (tx, mut rx) = mpsc::unbounded_channel::<(String, Value)>();

    EVENT_TX.set(tx).expect("event system already initialized");

    tokio::spawn(async move {
        while let Some((event, payload)) = rx.recv().await {
            let msg = serde_json::json!({
                "event": event,
                "payload": payload
            });
            let args: QList<QVariant> = vec![msg.to_string().into()].into();
            invoker.invoke_method_with_args("eventEmitted", &args);
        }
    });
}

pub fn emit<T: Serialize>(event: &str, payload: T) {
    let Ok(value) = serde_json::to_value(payload) else {
        return;
    };

    if let Some(tx) = EVENT_TX.get() {
        let _ = tx.send((event.to_string(), value));
    }
}