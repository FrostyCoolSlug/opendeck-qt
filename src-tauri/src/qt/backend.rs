//! General backend handler, handles invoke(), and events

use log::info;
use qtbridge::qtbridge_type_lib::{QList, QVariant};
use qtbridge::{QObjectHolder, qobject};
use serde_json::Value;
use crate::dispatch_command;
use crate::qt::events::init_event_system;

#[derive(Default)]
pub struct Backend {}

#[qobject(Singleton)]
impl Backend {
	#[qslot(qml_name = "init")]
	fn ensure_event_system(&mut self) {
		info!("[RUST] Backend init");
		static INIT: std::sync::Once = std::sync::Once::new();

		INIT.call_once(|| {
			let invoker = self.get_qml_method_invoker();
			init_event_system(invoker);
		});
	}

	// This is invoke() from JS, with async handling and response
	#[qslot(qml_name = "invokeFromJs")]
	fn invoke_from_js(&mut self, payload: String) {
		let parsed: Value = serde_json::from_str(&payload).unwrap_or_default();

		let command = parsed["command"].as_str().unwrap_or("").to_string();
		let args = parsed.get("args").cloned().unwrap_or_default();
		let request_id = parsed["requestId"].as_str().unwrap_or("").to_string();

		let invoker = self.get_qml_method_invoker();

		// Spawn this off, and call 'Ready' when it's finished.
		tokio::spawn(async move {
			let result = dispatch_command(&command, args).await;

			let response = match result {
				Ok(v) => serde_json::json!({
					"requestId": request_id,
					"status": "ok",
					"result": v
				}),
				Err(e) => serde_json::json!({
					"requestId": request_id,
					"status": "error",
					"error": e.to_string()
				}),
			};

			let args: QList<QVariant> = vec![response.to_string().into()].into();
			invoker.invoke_method_with_args("invokeResponse", &args);
		});
	}

	#[qsignal(qml_name = "invokeResponse")]
	fn invoke_response(&mut self, payload: String) {}

	#[qsignal(qml_name = "eventEmitted")]
	fn event_emitted(&mut self, payload: String) {}

	#[qslot(qml_name = "eventFromJs")]
	fn event_from_js(&mut self, payload: String) {
		println!("Event from JS: {}", payload);
	}
}
