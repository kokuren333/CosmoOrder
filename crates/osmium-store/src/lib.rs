//! SQLite learning state. Package text stays in the filesystem library.

pub mod runtime;

use osmium_core::{
    evaluation::evaluate,
    schema::{Diagnostic, DocumentKind, validate_document},
};
use osmium_package::{
    distribution::{Distribution, canonical_json, sha256},
    library::{InstalledPackage, Library},
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;
const APPLICATION_ID: i64 = 1330859337;
pub const STATE_VERSION: i64 = 1;

fn failure(code: &str, message: impl ToString) -> Vec<Diagnostic> {
    vec![Diagnostic {
        code: code.into(),
        severity: "error".into(),
        entity_type: None,
        entity_id: None,
        file: Some("state.sqlite".into()),
        line: None,
        column: None,
        path: String::new(),
        message: message.to_string(),
        suggestions: Vec::new(),
    }]
}
fn db(error: impl ToString) -> Vec<Diagnostic> {
    failure("OSM_IO", error)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptRequest {
    pub assessment_id: String,
    pub response: Value,
    pub request_id: String,
    pub duration_ms: Option<u64>,
    pub hints_used: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct AttemptResult {
    pub event: Value,
    pub replayed: bool,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ObjectiveProgress {
    pub objective_id: String,
    pub attempts: u64,
    pub correct: u64,
    pub accuracy: Option<f64>,
    pub last_score: Option<u8>,
    pub last_timestamp: Option<String>,
}

pub struct Store {
    connection: Connection,
    path: PathBuf,
    device_id: String,
}

impl Store {
    pub fn open(library: &Library) -> Result<Self> {
        for name in [
            "state.sqlite-wal",
            "state.sqlite-shm",
            "state.sqlite-journal",
        ] {
            library.checked_data_file(name)?;
        }
        let path = library.checked_data_file("state.sqlite")?;
        let mut connection = Connection::open(&path).map_err(db)?;
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(db)?;
        let application: i64 = connection
            .pragma_query_value(None, "application_id", |r| r.get(0))
            .map_err(db)?;
        if !(0..=STATE_VERSION).contains(&version) || (version > 0 && application != APPLICATION_ID)
        {
            return Err(failure(
                "OSM_STATE_VERSION",
                "unsupported or foreign SQLite database; no migration performed",
            ));
        }
        if version == 0 {
            let tables: i64 = connection
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'",
                    [],
                    |r| r.get(0),
                )
                .map_err(db)?;
            if tables != 0 || application != 0 {
                return Err(failure(
                    "OSM_STATE_VERSION",
                    "unrecognized unversioned database; preserving existing data",
                ));
            }
            let transaction = connection.transaction().map_err(db)?;
            transaction
                .execute_batch(include_str!("migrations/001.sql"))
                .map_err(db)?;
            transaction
                .execute(
                    "INSERT INTO settings(key,value) VALUES ('device_id',?1)",
                    [Uuid::new_v4().to_string()],
                )
                .map_err(db)?;
            transaction.commit().map_err(db)?;
        }
        connection
            .execute_batch(
                "PRAGMA foreign_keys=ON; PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL;",
            )
            .map_err(db)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(db)?;
        let device_id: String = connection
            .query_row(
                "SELECT value FROM settings WHERE key='device_id'",
                [],
                |r| r.get(0),
            )
            .map_err(db)?;
        Uuid::parse_str(&device_id).map_err(db)?;
        Ok(Self {
            connection,
            path,
            device_id,
        })
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn check_identity(&self, package_id: &str, version: &str, digest: &str) -> Result<()> {
        let existing: Option<String> = self
            .connection
            .query_row(
                "SELECT digest FROM packages WHERE package_id=?1 AND package_version=?2",
                params![package_id, version],
                |r| r.get(0),
            )
            .optional()
            .map_err(db)?;
        if existing.is_some_and(|old| old != digest) {
            return Err(failure(
                "OSM_VERSION_CONFLICT",
                "a different digest is already recorded for this package ID/version",
            ));
        }
        Ok(())
    }

    pub fn sync_packages(&mut self, packages: &[InstalledPackage]) -> Result<()> {
        for package in packages {
            self.check_identity(
                &package.package_id,
                &package.package_version,
                &package.digest,
            )?;
        }
        let transaction = self.connection.transaction().map_err(db)?;
        transaction
            .execute("UPDATE packages SET available=0", [])
            .map_err(db)?;
        for package in packages {
            transaction.execute("INSERT INTO packages(digest,package_id,package_version,metadata_json,available) VALUES (?1,?2,?3,?4,1) ON CONFLICT(digest) DO UPDATE SET available=1, metadata_json=excluded.metadata_json", params![package.digest, package.package_id, package.package_version, serde_json::to_string(package).map_err(db)?]).map_err(db)?;
        }
        transaction.commit().map_err(db)
    }

    pub fn submit(
        &mut self,
        package: &Distribution,
        request: &AttemptRequest,
    ) -> Result<AttemptResult> {
        let request_id = Uuid::parse_str(&request.request_id)
            .map_err(|_| failure("OSM_REQUEST_ID", "request_id must be a UUID"))?
            .to_string();
        let evaluation = evaluate(&package.model, &request.assessment_id, &request.response)?;
        if request.duration_ms.is_some_and(|n| n > 9007199254740991)
            || request.hints_used.is_some_and(|n| n > 9007199254740991)
        {
            return Err(failure(
                "OSM_RESPONSE",
                "observation exceeds the portable integer range",
            ));
        }
        let manifest = &package.model.documents().manifest;
        let timestamp = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(db)?;
        let event = json!({
            "event_schema_version":"0.1", "event_type":"assessment_attempt",
            "event_id":Uuid::new_v4().to_string(), "device_id":self.device_id, "request_id":request_id,
            "package_id":manifest["package_id"], "package_version":manifest["package_version"], "package_digest":package.digest,
            "assessment_id":evaluation.assessment_id, "assessment_revision":evaluation.assessment_revision,
            "assessment_hash":sha256(&canonical_json(&evaluation.assessment_snapshot)),
            "objective_ids":evaluation.objective_ids, "concept_ids":evaluation.concept_ids,
            "assessment_snapshot":evaluation.assessment_snapshot, "response":evaluation.response,
            "score":evaluation.score, "correct":evaluation.correct, "evaluator":evaluation.evaluator,
            "timestamp":timestamp, "duration_ms":request.duration_ms, "hints_used":request.hints_used
        });
        validate_event(&event)?;
        let transaction = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db)?;
        let existing: Option<String> = transaction
            .query_row(
                "SELECT event_json FROM events WHERE request_id=?1",
                [&request_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db)?;
        if let Some(existing) = existing {
            let existing: Value = serde_json::from_str(&existing).map_err(db)?;
            validate_event(&existing)?;
            for field in [
                "package_digest",
                "assessment_id",
                "assessment_hash",
                "response",
                "duration_ms",
                "hints_used",
            ] {
                if existing[field] != event[field] {
                    return Err(failure(
                        "OSM_REQUEST_CONFLICT",
                        "request_id was already used for a different attempt",
                    ));
                }
            }
            return Ok(AttemptResult {
                event: existing,
                replayed: true,
            });
        }
        transaction.execute("INSERT INTO events(event_id,request_id,package_digest,package_id,package_version,event_json) VALUES (?1,?2,?3,?4,?5,?6)", params![event["event_id"].as_str(),request_id,package.digest,manifest["package_id"].as_str(),manifest["package_version"].as_str(),serde_json::to_string(&event).map_err(db)?]).map_err(db)?;
        project(&transaction, &event)?;
        transaction.commit().map_err(db)?;
        Ok(AttemptResult {
            event,
            replayed: false,
        })
    }

    pub fn history(
        &self,
        package_id: &str,
        version: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Value>> {
        if limit == 0 || limit > 64 || offset > i64::MAX as usize {
            return Err(failure(
                "OSM_QUERY_LIMIT",
                "history requires limit 1..64 and a bounded offset",
            ));
        }
        let mut statement = self.connection.prepare("SELECT event_json FROM events WHERE package_id=?1 AND (?2 IS NULL OR package_version=?2) ORDER BY sequence DESC LIMIT ?3 OFFSET ?4").map_err(db)?;
        let rows = statement
            .query_map(
                params![package_id, version, limit as i64, offset as i64],
                |r| r.get::<_, String>(0),
            )
            .map_err(db)?;
        let mut bytes = 0;
        let mut events = Vec::new();
        for row in rows {
            let row = row.map_err(db)?;
            bytes += row.len();
            if bytes > 8 * 1024 * 1024 {
                return Err(failure(
                    "OSM_INPUT_LIMIT",
                    "history exceeds 8 MiB; request a smaller page",
                ));
            }
            let event = serde_json::from_str(&row).map_err(db)?;
            validate_event(&event)?;
            events.push(event);
        }
        Ok(events)
    }

    /// Read a bounded, newest-first history page across every Package. Events
    /// intentionally retain their Package IDs and assessment snapshots so
    /// answers remain inspectable after a Package payload is uninstalled.
    pub fn history_all(&self, limit: usize, offset: usize) -> Result<Vec<Value>> {
        if limit == 0 || limit > 64 || offset > i64::MAX as usize {
            return Err(failure(
                "OSM_QUERY_LIMIT",
                "history requires limit 1..64 and a bounded offset",
            ));
        }
        let mut statement = self
            .connection
            .prepare("SELECT event_json FROM events ORDER BY sequence DESC LIMIT ?1 OFFSET ?2")
            .map_err(db)?;
        let rows = statement
            .query_map(params![limit as i64, offset as i64], |r| {
                r.get::<_, String>(0)
            })
            .map_err(db)?;
        let mut bytes = 0;
        let mut events = Vec::new();
        for row in rows {
            let row = row.map_err(db)?;
            bytes += row.len();
            if bytes > 8 * 1024 * 1024 {
                return Err(failure(
                    "OSM_INPUT_LIMIT",
                    "history exceeds 8 MiB; request a smaller page",
                ));
            }
            let event = serde_json::from_str(&row).map_err(db)?;
            validate_event(&event)?;
            events.push(event);
        }
        Ok(events)
    }

    pub fn progress(&self, package: &Distribution) -> Result<Vec<ObjectiveProgress>> {
        let mut result = Vec::new();
        for objective in package.model.documents().objectives.as_array().unwrap() {
            let id = objective["id"].as_str().unwrap();
            let saved: Option<(i64,i64,u8,String)> = self.connection.query_row("SELECT attempts,correct,last_score,last_timestamp FROM progress WHERE package_digest=?1 AND objective_id=?2", params![package.digest,id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(db)?;
            let (attempts, correct, last_score, last_timestamp) = saved
                .map(|(a, c, s, t)| (a, c, Some(s), Some(t)))
                .unwrap_or((0, 0, None, None));
            let attempts = u64::try_from(attempts).map_err(db)?;
            let correct = u64::try_from(correct).map_err(db)?;
            result.push(ObjectiveProgress {
                objective_id: id.into(),
                attempts,
                correct,
                accuracy: (attempts > 0).then(|| correct as f64 / attempts as f64),
                last_score,
                last_timestamp,
            });
        }
        Ok(result)
    }

    pub fn rebuild_progress(&mut self) -> Result<usize> {
        let transaction = self.connection.transaction().map_err(db)?;
        transaction
            .execute("DELETE FROM progress", [])
            .map_err(db)?;
        let mut count = 0;
        {
            let mut statement = transaction
                .prepare("SELECT event_json FROM events ORDER BY sequence")
                .map_err(db)?;
            let rows = statement
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(db)?;
            for row in rows {
                let event = serde_json::from_str(&row.map_err(db)?).map_err(db)?;
                validate_event(&event)?;
                project(&transaction, &event)?;
                count += 1;
            }
        }
        transaction.commit().map_err(db)?;
        Ok(count)
    }

    pub fn export_state(&mut self, output: &std::path::Path) -> Result<usize> {
        use std::io::Write;
        let mut temporary = new_output(output)?;
        let transaction = self.connection.transaction().map_err(db)?;
        let header = json!({"record_type":"osmium-state", "export_version":"0.1", "event_schema_version":"0.1", "device_id":self.device_id});
        writeln!(temporary, "{header}").map_err(db)?;
        let mut count = 0;
        {
            let mut statement = transaction
                .prepare("SELECT event_json FROM events ORDER BY sequence")
                .map_err(db)?;
            let rows = statement
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(db)?;
            for row in rows {
                let event = serde_json::from_str(&row.map_err(db)?).map_err(db)?;
                validate_event(&event)?;
                let record = json!({"record_type":"learning_event", "event":event});
                writeln!(temporary, "{record}").map_err(db)?;
                count += 1;
            }
        }
        transaction.commit().map_err(db)?;
        temporary.as_file().sync_all().map_err(db)?;
        temporary
            .persist_noclobber(output)
            .map_err(|e| db(e.error))?;
        Ok(count)
    }

    pub fn backup(&self, output: &std::path::Path) -> Result<()> {
        let temporary = new_output(output)?;
        self.connection
            .backup(rusqlite::MAIN_DB, temporary.path(), None)
            .map_err(db)?;
        temporary.as_file().sync_all().map_err(db)?;
        temporary
            .persist_noclobber(output)
            .map_err(|e| db(e.error))?;
        Ok(())
    }
}

fn new_output(output: &std::path::Path) -> Result<tempfile::NamedTempFile> {
    if std::fs::symlink_metadata(output).is_ok() {
        return Err(failure("OSM_OUTPUT_EXISTS", "output already exists"));
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    tempfile::NamedTempFile::new_in(parent).map_err(db)
}

fn validate_event(event: &Value) -> Result<()> {
    let errors = validate_document(DocumentKind::LearningEvent, event);
    if !errors.is_empty() {
        return Err(errors);
    }
    let snapshot = &event["assessment_snapshot"];
    if event["assessment_hash"] != sha256(&canonical_json(snapshot))
        || event["assessment_id"] != snapshot["id"]
        || event["assessment_revision"] != snapshot["revision"]
        || event["objective_ids"] != snapshot["measures"]
    {
        return Err(failure(
            "OSM_EVENT_INTEGRITY",
            "event identity or snapshot hash is inconsistent",
        ));
    }
    let response = &event["response"];
    let valid = match snapshot["response"]["type"].as_str() {
        Some("boolean") => response.is_boolean(),
        Some("single_select") => snapshot["response"]["options"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["id"] == *response),
        _ => false,
    };
    let correct = *response == snapshot["evaluation"]["answer"];
    if !valid
        || event["score"] != json!(u8::from(correct))
        || event.get("correct").is_some_and(|v| *v != correct)
    {
        return Err(failure(
            "OSM_EVENT_INTEGRITY",
            "event response and score disagree",
        ));
    }
    Ok(())
}

fn project(transaction: &Transaction<'_>, event: &Value) -> Result<()> {
    for objective in event["objective_ids"].as_array().unwrap() {
        transaction.execute("INSERT INTO progress(package_digest,objective_id,attempts,correct,last_score,last_timestamp,last_event_id) VALUES (?1,?2,1,?3,?3,?4,?5) ON CONFLICT(package_digest,objective_id) DO UPDATE SET attempts=progress.attempts+1,correct=progress.correct+excluded.correct,last_score=excluded.last_score,last_timestamp=excluded.last_timestamp,last_event_id=excluded.last_event_id", params![event["package_digest"].as_str(),objective.as_str(),event["score"].as_i64(),event["timestamp"].as_str(),event["event_id"].as_str()]).map_err(db)?;
    }
    Ok(())
}
