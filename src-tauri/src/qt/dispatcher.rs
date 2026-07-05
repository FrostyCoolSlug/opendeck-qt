use serde::Serialize;
use serde_json::Value;

pub struct Wrap<T>(pub T);

// Fallback: any plain Serialize type
pub trait ViaSerialize {
    fn into_dispatch_result(&self) -> Result<Value, String>;
}
impl<T: Serialize> ViaSerialize for Wrap<T> {
    fn into_dispatch_result(&self) -> Result<Value, String> {
        serde_json::to_value(&self.0).map_err(|e| e.to_string())
    }
}

// Specific: Result<T, E> — matched preferentially via autoref
pub trait ViaResult {
    fn into_dispatch_result(&self) -> Result<Value, String>;
}
impl<T: Serialize, E: std::fmt::Display> ViaResult for &Wrap<Result<T, E>> {
    fn into_dispatch_result(&self) -> Result<Value, String> {
        match &self.0 {
            Ok(v) => serde_json::to_value(v).map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        }
    }
}