CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT;
CREATE TABLE packages (
    digest TEXT PRIMARY KEY,
    package_id TEXT NOT NULL,
    package_version TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    available INTEGER NOT NULL CHECK(available IN (0,1)),
    UNIQUE(package_id, package_version)
) STRICT;
CREATE TABLE events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    request_id TEXT NOT NULL UNIQUE,
    package_digest TEXT NOT NULL REFERENCES packages(digest),
    package_id TEXT NOT NULL,
    package_version TEXT NOT NULL,
    event_json TEXT NOT NULL
) STRICT;
CREATE INDEX events_package ON events(package_id, package_version, sequence);
CREATE TRIGGER events_no_update BEFORE UPDATE ON events BEGIN
    SELECT RAISE(ABORT, 'learning events are append-only');
END;
CREATE TRIGGER events_no_delete BEFORE DELETE ON events BEGIN
    SELECT RAISE(ABORT, 'learning events are append-only');
END;
CREATE VIEW assessment_attempts AS SELECT sequence, event_id, package_id,
    package_version, json_extract(event_json, '$.assessment_id') AS assessment_id,
    json_extract(event_json, '$.score') AS score, event_json FROM events;
CREATE TABLE progress (
    package_digest TEXT NOT NULL REFERENCES packages(digest),
    objective_id TEXT NOT NULL,
    attempts INTEGER NOT NULL CHECK(attempts > 0),
    correct INTEGER NOT NULL CHECK(correct >= 0 AND correct <= attempts),
    last_score INTEGER NOT NULL CHECK(last_score IN (0,1)),
    last_timestamp TEXT NOT NULL,
    last_event_id TEXT NOT NULL,
    PRIMARY KEY(package_digest, objective_id)
) STRICT;
PRAGMA application_id = 1330859337;
PRAGMA user_version = 1;
