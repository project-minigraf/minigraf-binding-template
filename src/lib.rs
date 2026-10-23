//! Minigraf binding template — UniFFI scaffolding.
//!
//! CUSTOMISATION GUIDE:
//! 1. Rename the crate in Cargo.toml (name, description).
//! 2. Pin minigraf to the exact version you want to bind (`minigraf = "=2.x.y"`).
//! 3. Implement any extra error variants or wrapper types your language needs.
//! 4. Add your language tooling (e.g. java/, android/, Sources/, etc.).
//! 5. Wire up a release.yml that receives `core-release` dispatch from
//!    the minigraf cascade and calls `cargo publish` / your language publisher.
//!
//! Why depend on `minigraf` directly and not `minigraf-ffi`?
//! UniFFI's `setup_scaffolding!()` macro generates extern "C" symbols in the
//! *final* cdylib crate. Re-exporting from a crate that already called it
//! causes duplicate symbol errors at link time. Always embed the scaffolding
//! in your own crate and depend on `minigraf` core.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
    )
)]

use minigraf::{QueryResult, Value};
use std::sync::{Arc, Mutex};

uniffi::setup_scaffolding!();

// ─── Error type ──────────────────────────────────────────────────────────────
// CUSTOMISE: add variants for any language-specific error categories.

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MiniGrafError {
    #[error("storage error: {msg}")]
    Storage { msg: String },
    #[error("query error: {msg}")]
    Query { msg: String },
    #[error("parse error: {msg}")]
    Parse { msg: String },
    #[error("unknown error: {msg}")]
    Other { msg: String },
}

// minigraf v2 returns a typed `MinigrafError`: map on its category, not on
// message text. Its Display carries the stable code ("[PRS-001] ..."), so the
// message each variant holds does too.
impl From<minigraf::MinigrafError> for MiniGrafError {
    fn from(e: minigraf::MinigrafError) -> Self {
        let msg = e.to_string();
        match e.category() {
            minigraf::ErrorCategory::Parser => MiniGrafError::Parse { msg },
            minigraf::ErrorCategory::Storage | minigraf::ErrorCategory::Wal => {
                MiniGrafError::Storage { msg }
            }
            minigraf::ErrorCategory::Query => MiniGrafError::Query { msg },
            // ErrorCategory is #[non_exhaustive]: new categories land here.
            minigraf::ErrorCategory::Api | minigraf::ErrorCategory::Internal | _ => {
                MiniGrafError::Other { msg }
            }
        }
    }
}

// ─── Database object ─────────────────────────────────────────────────────────
// CUSTOMISE: add methods your language needs (e.g. async wrappers, transactions).

#[derive(uniffi::Object)]
pub struct MiniGrafDb {
    inner: Arc<Mutex<minigraf::Minigraf>>,
}

#[uniffi::export]
impl MiniGrafDb {
    #[uniffi::constructor]
    pub fn open(path: String) -> Result<Arc<Self>, MiniGrafError> {
        let db = minigraf::Minigraf::open(&path).map_err(MiniGrafError::from)?;
        Ok(Arc::new(Self {
            inner: Arc::new(Mutex::new(db)),
        }))
    }

    #[uniffi::constructor]
    pub fn open_in_memory() -> Result<Arc<Self>, MiniGrafError> {
        let db = minigraf::Minigraf::in_memory().map_err(MiniGrafError::from)?;
        Ok(Arc::new(Self {
            inner: Arc::new(Mutex::new(db)),
        }))
    }

    pub fn execute(&self, datalog: String) -> Result<String, MiniGrafError> {
        let result = self
            .inner
            .lock()
            .map_err(|_| MiniGrafError::Other {
                msg: "mutex poisoned".into(),
            })?
            .execute(&datalog)
            .map_err(MiniGrafError::from)?;
        Ok(query_result_to_json(result))
    }

    pub fn checkpoint(&self) -> Result<(), MiniGrafError> {
        self.inner
            .lock()
            .map_err(|_| MiniGrafError::Other {
                msg: "mutex poisoned".into(),
            })?
            .checkpoint()
            .map_err(MiniGrafError::from)
    }
}

// ─── JSON serialisation ───────────────────────────────────────────────────────

fn value_to_json(v: &Value) -> serde_json::Value {
    use serde_json::Value as JVal;
    match v {
        Value::String(s) => JVal::String(s.clone()),
        Value::Integer(i) => JVal::Number((*i).into()),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(JVal::Number)
            .unwrap_or(JVal::Null),
        Value::Boolean(b) => JVal::Bool(*b),
        Value::Ref(uuid) => JVal::String(uuid.to_string()),
        Value::Keyword(k) => JVal::String(k.clone()),
        Value::Null => JVal::Null,
    }
}

fn query_result_to_json(result: QueryResult) -> String {
    use serde_json::json;
    let val = match result {
        QueryResult::Transacted(tx_id) => json!({"transacted": tx_id}),
        QueryResult::Retracted(tx_id) => json!({"retracted": tx_id}),
        QueryResult::Ok => json!({"ok": true}),
        QueryResult::QueryResults { vars, results } => {
            let rows: Vec<Vec<serde_json::Value>> = results
                .iter()
                .map(|row| row.iter().map(value_to_json).collect())
                .collect();
            json!({"variables": vars, "results": rows})
        }
    };
    val.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_succeeds() {
        MiniGrafDb::open_in_memory().expect("open_in_memory");
    }

    #[test]
    fn execute_transact_returns_json() {
        let db = MiniGrafDb::open_in_memory().expect("open");
        let json = db
            .execute(r#"(transact [[:alice :name "Alice"]])"#.into())
            .expect("execute");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert!(v.get("transacted").is_some());
    }

    #[test]
    fn execute_query_returns_results() {
        let db = MiniGrafDb::open_in_memory().expect("open");
        db.execute(r#"(transact [[:alice :name "Alice"]])"#.into())
            .expect("transact");
        let json = db
            .execute(r#"(query [:find ?n :where [?e :name ?n]])"#.into())
            .expect("query");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert_eq!(v["results"][0][0], "Alice");
    }

    #[test]
    fn execute_invalid_datalog_returns_parse_error_with_code() {
        let db = MiniGrafDb::open_in_memory().expect("open");
        let result = db.execute("this is not datalog !!!".into());
        match result {
            Err(MiniGrafError::Parse { msg }) => {
                assert!(msg.starts_with("[PRS-"), "expected error code in {msg:?}");
            }
            other => panic!("expected Parse error, got {other:?}"),
        }
    }

    #[test]
    fn open_file_backed_roundtrip() {
        let dir = std::env::temp_dir().join(format!("minigraf_template_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("test.graph");
        let path_str = path.to_str().expect("utf8 path").to_string();

        {
            let db = MiniGrafDb::open(path_str.clone()).expect("open");
            db.execute(r#"(transact [[:alice :name "Alice"]])"#.into())
                .expect("transact");
            db.checkpoint().expect("checkpoint");
        }

        let db = MiniGrafDb::open(path_str).expect("reopen");
        let json = db
            .execute(r#"(query [:find ?n :where [?e :name ?n]])"#.into())
            .expect("query");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert_eq!(v["results"][0][0], "Alice");
        drop(db);
        let _ = std::fs::remove_dir_all(dir);
    }
}
