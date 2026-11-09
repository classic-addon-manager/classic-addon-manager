//! Developer endpoints of the server (`/dev/addon/*`). Calls block: the app runs them off the UI thread. Each
//! endpoint decodes the `data` of its response into the wire types of [`types`], with the rules of `parse`. A reply
//! that doesn't decode is an [`ApiError::Decode`].
//!
//! What the user reads for a failure is decided here, per endpoint ([`Messages`]). The documented `400`/`409` replies
//! of the submission endpoints become outcomes the form matches on ([`ValidateResult`], [`SubmitAddonResult`],
//! [`WithdrawResult`]), so nothing outside this module looks at the server's wording.
pub mod declaration;
mod parse;
pub mod types;

use std::path::Path;

use serde::de::DeserializeOwned;
use serde_json::json;
use thiserror::Error;

use self::{
    declaration::{PublishFormState, form_to_values, text_of},
    parse::{Acknowledgement, StoredValues, Verdict, group_validation_errors},
    types::*,
};
use super::{ApiError, Body, FormPart, Method, encode_uri_component, request};

const SOURCE_UNAVAILABLE: &str = "This source is unavailable.";

/// Shown when the server's schema is newer than this app understands.
pub const UNSUPPORTED_SCHEMA: &str = "This app needs an update to edit addon declarations.";

/// The sentences one endpoint shows for each way a call can fail.
pub struct Messages {
    /// Not signed in.
    sign_in: &'static str,
    /// The addon, submission or source doesn't exist or isn't the caller's, `None` where the endpoint has no such case.
    unavailable: Option<&'static str>,
    /// Anything the server didn't explain itself: no connection, a reply of the wrong shape, an error without a message.
    unexpected: &'static str,
}

impl Messages {
    /// The sentence for `error`. A rejection the server explained shows the server's own words.
    pub fn message(&self, error: &ApiError) -> String {
        let text = match error {
            ApiError::Unauthorized => self.sign_in,
            ApiError::NotFound => self.unavailable.unwrap_or(self.unexpected),
            ApiError::Conflict { .. } | ApiError::Rejected { .. } => error.server_message().unwrap_or(self.unexpected),
            ApiError::Network(_) | ApiError::Validation(_) | ApiError::Decode(_) => self.unexpected,
        };
        text.to_owned()
    }
}

pub const SCHEMA: Messages = Messages {
    sign_in: "Sign in to edit addon declarations.",
    unavailable: None,
    unexpected: "Unexpected schema response.",
};

pub const SOURCES: Messages =
    Messages { sign_in: "Sign in to view your addons.", unavailable: None, unexpected: "Unexpected sources response." };

pub const STATS: Messages = Messages {
    sign_in: "Sign in to view addon statistics.",
    unavailable: Some("These statistics are unavailable."),
    unexpected: "Unexpected statistics response.",
};

pub const SUBMISSION: Messages = Messages {
    sign_in: "Sign in to view this submission.",
    unavailable: Some("This submission is unavailable."),
    unexpected: "Unexpected submission response.",
};

pub const VALUES: Messages = Messages {
    sign_in: "Sign in to load addon values.",
    unavailable: Some(SOURCE_UNAVAILABLE),
    unexpected: "Unexpected values response.",
};

const SAVE: Messages = Messages {
    sign_in: "Sign in to submit an addon.",
    unavailable: Some(SOURCE_UNAVAILABLE),
    unexpected: "Unexpected save response.",
};

const WITHDRAW: Messages = Messages {
    sign_in: "Sign in to withdraw a submission.",
    unavailable: Some(SOURCE_UNAVAILABLE),
    unexpected: "Unexpected withdraw response.",
};

// ---- documented server replies --------------------------------------------------------------------------------------

/// `400` messages for an id or query the server couldn't parse. For the user such a record doesn't exist, the same as
/// after a `404`.
const INVALID_ADDON_UUID: &str = "invalid addon uuid";
const INVALID_VALUES_QUERY: &str = "invalid values query";
const INVALID_SUBMISSION_ID: &str = "invalid submission id";

/// `409` messages of the submission endpoints. The reply carries the id of the submission in question.
const SUBMISSION_ALREADY_OPEN: &str = "submission already open";
const SUBMISSION_NOT_OPEN: &str = "submission not open";

/// Sends a request whose reply must carry `data` of type `T`.
fn fetch<T: DeserializeOwned>(method: Method, path: &str, body: Body) -> Result<T, ApiError> {
    request::<Option<T>>(method, path, body)?.ok_or_else(|| ApiError::Decode("reply without data".into()))
}

/// A `400` saying the id or query was malformed (`message`) is a record that doesn't exist; anything else is kept.
fn malformed_is_not_found(error: ApiError, message: &str) -> ApiError {
    match error {
        ApiError::Rejected { status: 400, message: m } if m == message => ApiError::NotFound,
        other => other,
    }
}

/// What a `409` of the submission endpoints says about the submission it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SubmissionConflict {
    /// The caller already has an open submission with the same name; carries that submission's id.
    AlreadyOpen(i64),
    /// The submission is closed (approved or withdrawn) and can't change any more; carries its id.
    NotOpen(i64),
}

impl SubmissionConflict {
    fn of(error: &ApiError) -> Option<Self> {
        let ApiError::Conflict { id, message } = error else { return None };
        match message.as_str() {
            SUBMISSION_ALREADY_OPEN => Some(Self::AlreadyOpen(*id)),
            SUBMISSION_NOT_OPEN => Some(Self::NotOpen(*id)),
            _ => None,
        }
    }
}

// ---- schema ---------------------------------------------------------------------------------------------------------

/// Fetches the declaration schema.
pub fn schema() -> Result<SchemaDoc, ApiError> {
    fetch(Method::Get, "/dev/addon/schema", Body::None)
}

/// An empty value for every schema field: unchecked, empty text or an empty list.
pub fn schema_zero_values(schema: &AddonSchema) -> DeclarationValues {
    schema.fields.iter().map(|f| (f.key.clone(), WireValue::empty(f.widget))).collect()
}

// ---- reads ----------------------------------------------------------------------------------------------------------

/// The caller's addons and submissions.
pub fn sources() -> Result<AddonSources, ApiError> {
    fetch(Method::Get, "/dev/addon/sources", Body::None)
}

pub fn stats(uuid: &str) -> Result<AddonDeveloperStats, ApiError> {
    let path = format!("/dev/addon/{}/stats", encode_uri_component(uuid));
    fetch(Method::Get, &path, Body::None).map_err(|e| malformed_is_not_found(e, INVALID_ADDON_UUID))
}

pub fn submission(id: i64) -> Result<SubmissionDetail, ApiError> {
    let path = format!("/dev/addon/submissions/{id}");
    fetch(Method::Get, &path, Body::None).map_err(|e| malformed_is_not_found(e, INVALID_SUBMISSION_ID))
}

/// The stored values of `source`, read through the declaration `schema`.
pub fn values(source: &EditorSource, schema: &AddonSchema) -> Result<SourceValues, ApiError> {
    let query = match source {
        EditorSource::New => "source=new".to_string(),
        EditorSource::Addon { uuid } => format!("addon_uuid={}", encode_uri_component(uuid)),
        EditorSource::Submission { id } => format!("submission_id={id}"),
    };
    let path = format!("/dev/addon/values?{query}");
    let stored: StoredValues =
        fetch(Method::Get, &path, Body::None).map_err(|e| malformed_is_not_found(e, INVALID_VALUES_QUERY))?;
    stored.read(schema).map_err(ApiError::Decode)
}

// ---- icon upload ----------------------------------------------------------------------------------------------------

const UPLOAD_FAILED: &str = "Icon upload failed.";

/// Why an icon could not be uploaded.
#[derive(Debug, Error)]
pub enum UploadError {
    #[error("can't read the icon file: {0}")]
    Unreadable(#[from] std::io::Error),
    #[error(transparent)]
    Api(#[from] ApiError),
}

impl UploadError {
    /// What the icon dialog shows: the server's own sentence when it refused the icon itself, a generic failure
    /// otherwise.
    pub fn message(&self) -> String {
        let refusal = match self {
            UploadError::Api(ApiError::Validation(errors)) => errors.iter().find(|e| e.field == "icon"),
            _ => None,
        };
        refusal.map_or(UPLOAD_FAILED, |e| e.message.as_str()).to_owned()
    }
}

/// Stages the PNG at `path` as an addon icon: a multipart POST with the file as the `icon` part.
pub fn upload_icon(path: &Path) -> Result<UploadedIcon, UploadError> {
    let data = std::fs::read(path)?;
    let filename = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "icon.png".into());
    let part = FormPart { name: "icon".into(), filename, content_type: "image/png".into(), data };
    Ok(fetch(Method::Post, "/dev/addon/icon", Body::Form(vec![part]))?)
}

// ---- declaration writes ---------------------------------------------------------------------------------------------

fn editor_validate_path(submission_id: Option<i64>) -> String {
    match submission_id {
        None => "/dev/addon/validate".into(),
        Some(id) => format!("/dev/addon/submissions/{id}/validate"),
    }
}

fn editor_save_request(submission_id: Option<i64>) -> (Method, String) {
    match submission_id {
        None => (Method::Post, "/dev/addon/submit".into()),
        Some(id) => (Method::Put, format!("/dev/addon/submissions/{id}")),
    }
}

fn editor_withdraw_path(submission_id: i64) -> String {
    format!("/dev/addon/submissions/{submission_id}/withdraw")
}

/// The keys a validation error can be attributed to in the form: the schema fields and the icon.
fn declaration_error_keys() -> Vec<&'static str> {
    SCHEMA_FIELD_KEYS.iter().copied().chain(["icon"]).collect()
}

/// Runs the server's checks on a declaration without storing it. A declaration with problems is
/// [`ApiError::Validation`], the same as when [`save`] refuses it.
fn validate(values: &DeclarationValues, submission_id: Option<i64>) -> Result<(), ApiError> {
    let verdict: Verdict = fetch(Method::Post, &editor_validate_path(submission_id), Body::Json(json!(values)))
        .map_err(|e| malformed_is_not_found(e, INVALID_SUBMISSION_ID))?;
    if verdict.valid { Ok(()) } else { Err(ApiError::Validation(verdict.errors)) }
}

/// Stores a declaration as a new submission, or as the next revision of `submission_id`. Returns the stored id.
fn save(values: &DeclarationValues, submission_id: Option<i64>) -> Result<i64, ApiError> {
    let (method, path) = editor_save_request(submission_id);
    let saved: Acknowledgement = fetch(method, &path, Body::Json(json!(values)))
        .map_err(|e| malformed_is_not_found(e, INVALID_SUBMISSION_ID))?;
    Ok(saved.id)
}

/// Closes an open submission.
fn withdraw(submission_id: i64) -> Result<(), ApiError> {
    let reply: Acknowledgement = fetch(Method::Post, &editor_withdraw_path(submission_id), Body::None)
        .map_err(|e| malformed_is_not_found(e, INVALID_SUBMISSION_ID))?;
    match reply.status.as_deref() {
        Some("withdrawn") => Ok(()),
        status => Err(ApiError::Decode(format!("withdraw acknowledged with status {status:?}"))),
    }
}

/// The id of the caller's open or rejected submission named `name`, if any: the user resumes it instead of creating a
/// duplicate.
fn find_actionable_submission(name: &str) -> Result<Option<i64>, ApiError> {
    let wanted = name.trim().to_lowercase();
    let found = sources()?
        .submissions
        .into_iter()
        .find(|s| s.name.trim().to_lowercase() == wanted && matches!(s.status.as_deref(), Some("open" | "rejected")));
    Ok(found.map(|s| s.id))
}

// ---- icon rules -----------------------------------------------------------------------------------------------------

// Client-side icon rules. UX only: the server re-checks everything.
const ICON_MAX_BYTES: u64 = 3 * 1024 * 1024;
const ICON_MIN_DIMENSION: u32 = 50;

/// The problems with a picked icon file, empty when it is fine.
pub fn validate_icon_file(path: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let claims_png = path.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase().ends_with(".png"));
    if !claims_png {
        errors.push("Icon must be a PNG file.".to_string());
    }
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size > ICON_MAX_BYTES {
        errors.push("Icon must be 3 MB or smaller.".to_string());
    }
    match read_png_dimensions(path) {
        None => {
            if claims_png {
                errors.push("Icon must be a valid PNG file.".to_string());
            }
        }
        Some((width, height)) => {
            if width != height {
                errors.push("Icon must be square (1:1).".to_string());
            }
            if width < ICON_MIN_DIMENSION || height < ICON_MIN_DIMENSION {
                errors.push(format!("Icon must be at least {ICON_MIN_DIMENSION}×{ICON_MIN_DIMENSION} pixels."));
            }
        }
    }
    errors
}

fn read_png_dimensions(path: &Path) -> Option<(u32, u32)> {
    use std::io::Read;
    let mut buf = Vec::new();
    std::fs::File::open(path).ok()?.take(512 * 1024).read_to_end(&mut buf).ok()?;
    png_dimensions(&buf)
}

/// Detects PNG by signature (a file renamed to .png holding JPEG/WebP data is rejected) and reads IHDR.
pub fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 24 || bytes[..8] != SIG {
        return None;
    }
    // First chunk is IHDR (Apple's CgBI PNGs put CgBI first and IHDR second).
    let ihdr_at = if &bytes[12..16] == b"IHDR" {
        16
    } else if &bytes[12..16] == b"CgBI" && bytes.len() >= 48 && &bytes[40..44] == b"IHDR" {
        44
    } else {
        return None;
    };
    let w = u32::from_be_bytes(bytes.get(ihdr_at..ihdr_at + 4)?.try_into().ok()?);
    let h = u32::from_be_bytes(bytes.get(ihdr_at + 4..ihdr_at + 8)?.try_into().ok()?);
    Some((w, h))
}

// ---- outcomes -------------------------------------------------------------------------------------------------------

/// Outcome of checking a declaration before it is submitted.
#[derive(Clone, Debug, PartialEq)]
pub enum ValidateResult {
    Valid,
    Invalid {
        fields: FieldErrors,
        other: Vec<String>,
    },
    /// The caller has an open or rejected submission under this name: resume it instead.
    AlreadyOpen(i64),
    /// The submission being edited was closed meanwhile.
    NotOpen(i64),
    /// No verdict: the check itself failed.
    Failed(ApiError),
}

/// Outcome of submitting a declaration.
#[derive(Clone, Debug, PartialEq)]
pub enum SubmitAddonResult {
    Submitted(i64),
    /// The caller has an open or rejected submission under this name: resume it instead.
    AlreadyOpen(i64),
    Invalid {
        fields: FieldErrors,
        other: Vec<String>,
    },
    /// The submission being edited was closed meanwhile.
    NotOpen(i64),
    /// Not submitted; the message says why.
    Error(String),
}

/// Outcome of withdrawing a submission.
#[derive(Clone, Debug, PartialEq)]
pub enum WithdrawResult {
    Withdrawn,
    /// The submission was closed (approved or withdrawn) before.
    NotOpen(i64),
    /// The submission no longer exists.
    NotFound(String),
    /// Not withdrawn; the message says why.
    Error(String),
}

const NAME_ALREADY_TAKEN: &str = "name is already taken";

fn is_name_already_taken(submission_id: Option<i64>, fields: &FieldErrors, other: &[String]) -> bool {
    if submission_id.is_some() {
        return false;
    }
    fields.get("name").into_iter().flatten().chain(other).any(|m| m.trim().to_lowercase() == NAME_ALREADY_TAKEN)
}

fn validate_outcome(result: Result<(), ApiError>) -> ValidateResult {
    match result {
        Ok(()) => ValidateResult::Valid,
        Err(ApiError::Validation(errors)) => {
            let (fields, other) = group_validation_errors(&errors, &declaration_error_keys());
            ValidateResult::Invalid { fields, other }
        }
        Err(e) => match SubmissionConflict::of(&e) {
            Some(SubmissionConflict::NotOpen(id)) => ValidateResult::NotOpen(id),
            _ => ValidateResult::Failed(e),
        },
    }
}

fn save_outcome(result: Result<i64, ApiError>) -> SubmitAddonResult {
    match result {
        Ok(id) => SubmitAddonResult::Submitted(id),
        Err(ApiError::Validation(errors)) => {
            let (fields, other) = group_validation_errors(&errors, &declaration_error_keys());
            SubmitAddonResult::Invalid { fields, other }
        }
        Err(e) => match SubmissionConflict::of(&e) {
            Some(SubmissionConflict::AlreadyOpen(id)) => SubmitAddonResult::AlreadyOpen(id),
            Some(SubmissionConflict::NotOpen(id)) => SubmitAddonResult::NotOpen(id),
            None => SubmitAddonResult::Error(SAVE.message(&e)),
        },
    }
}

fn withdraw_outcome(result: Result<(), ApiError>) -> WithdrawResult {
    match result {
        Ok(()) => WithdrawResult::Withdrawn,
        Err(e @ ApiError::NotFound) => WithdrawResult::NotFound(WITHDRAW.message(&e)),
        Err(e) => match SubmissionConflict::of(&e) {
            Some(SubmissionConflict::NotOpen(id)) => WithdrawResult::NotOpen(id),
            _ => WithdrawResult::Error(WITHDRAW.message(&e)),
        },
    }
}

/// Checks a declaration with the server. A name another submission of the caller already uses resolves to that
/// submission instead of a field error.
pub fn validate_addon(form: PublishFormState, submission_id: Option<i64>) -> ValidateResult {
    let values = form_to_values(form);
    match validate_outcome(validate(&values, submission_id)) {
        ValidateResult::Invalid { fields, other } if is_name_already_taken(submission_id, &fields, &other) => {
            match find_actionable_submission(&text_of(&values, "name")) {
                Ok(Some(id)) => ValidateResult::AlreadyOpen(id),
                Ok(None) => ValidateResult::Invalid { fields, other },
                Err(e) => ValidateResult::Failed(e),
            }
        }
        outcome => outcome,
    }
}

/// Submits a declaration: a new submission, or the next revision of `submission_id`. A new one that reuses the name of
/// an open or rejected submission of the caller resolves to that submission instead.
pub fn submit_addon(form: PublishFormState, submission_id: Option<i64>) -> SubmitAddonResult {
    if submission_id.is_none() {
        match find_actionable_submission(&text_of(&form.values, "name")) {
            Ok(Some(id)) => return SubmitAddonResult::AlreadyOpen(id),
            Ok(None) => {}
            Err(e) => return SubmitAddonResult::Error(SOURCES.message(&e)),
        }
    }
    save_outcome(save(&form_to_values(form), submission_id))
}

/// Withdraws an open submission.
pub fn withdraw_submission(id: i64) -> WithdrawResult {
    withdraw_outcome(withdraw(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::FieldError;

    fn png_bytes(w: u32, h: u32) -> Vec<u8> {
        let mut b = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R'];
        b.extend_from_slice(&w.to_be_bytes());
        b.extend_from_slice(&h.to_be_bytes());
        b.extend_from_slice(&[8, 6, 0, 0, 0]);
        b
    }

    fn rejected(status: u16, message: &str) -> ApiError {
        ApiError::Rejected { status, message: message.into() }
    }

    fn conflict(id: i64, message: &str) -> ApiError {
        ApiError::Conflict { id, message: message.into() }
    }

    fn field_error(field: &str, message: &str) -> FieldError {
        FieldError { field: field.into(), message: message.into() }
    }

    #[test]
    fn png_header() {
        assert_eq!(png_dimensions(&png_bytes(64, 48)), Some((64, 48)));
        assert_eq!(png_dimensions(b"\xFF\xD8\xFF\xE0 not a png at all, a jpeg"), None);
        assert_eq!(png_dimensions(&[]), None);
    }

    #[test]
    fn icon_rules() {
        let dir = std::env::temp_dir().join(format!("cam-dev-icon-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ok = dir.join("ok.png");
        std::fs::write(&ok, png_bytes(64, 64)).unwrap();
        assert!(validate_icon_file(&ok).is_empty());
        let small = dir.join("small.png");
        std::fs::write(&small, png_bytes(32, 32)).unwrap();
        assert_eq!(validate_icon_file(&small), vec!["Icon must be at least 50×50 pixels."]);
        let rect = dir.join("rect.png");
        std::fs::write(&rect, png_bytes(64, 80)).unwrap();
        assert_eq!(validate_icon_file(&rect), vec!["Icon must be square (1:1)."]);
        let fake = dir.join("fake.png");
        std::fs::write(&fake, b"\xFF\xD8\xFF jpeg").unwrap();
        assert_eq!(validate_icon_file(&fake), vec!["Icon must be a valid PNG file."]);
        let jpg = dir.join("x.jpg");
        std::fs::write(&jpg, png_bytes(64, 64)).unwrap();
        assert_eq!(validate_icon_file(&jpg), vec!["Icon must be a PNG file."]);
        let big = dir.join("big.png");
        let mut v = png_bytes(64, 64);
        v.resize(3 * 1024 * 1024 + 1, 0);
        std::fs::write(&big, v).unwrap();
        assert_eq!(validate_icon_file(&big), vec!["Icon must be 3 MB or smaller."]);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn paths() {
        assert_eq!(editor_validate_path(None), "/dev/addon/validate");
        assert_eq!(editor_validate_path(Some(4)), "/dev/addon/submissions/4/validate");
        assert_eq!(editor_save_request(None), (Method::Post, "/dev/addon/submit".to_string()));
        assert_eq!(editor_save_request(Some(4)), (Method::Put, "/dev/addon/submissions/4".to_string()));
        assert_eq!(editor_withdraw_path(4), "/dev/addon/submissions/4/withdraw");
    }

    #[test]
    fn name_taken() {
        let mut fields = FieldErrors::new();
        fields.insert("name".into(), vec!["  Name is already taken ".into()]);
        assert!(is_name_already_taken(None, &fields, &[]));
        assert!(!is_name_already_taken(Some(1), &fields, &[]));
        assert!(is_name_already_taken(None, &FieldErrors::new(), &["name is already taken".to_string()]));
        assert!(!is_name_already_taken(None, &FieldErrors::new(), &["other".to_string()]));
    }

    #[test]
    fn failures_read_per_endpoint() {
        assert_eq!(STATS.message(&ApiError::Unauthorized), "Sign in to view addon statistics.");
        assert_eq!(STATS.message(&ApiError::NotFound), "These statistics are unavailable.");
        assert_eq!(STATS.message(&ApiError::Network("timed out".into())), "Unexpected statistics response.");
        assert_eq!(STATS.message(&ApiError::Decode("malformed statistics".into())), "Unexpected statistics response.");
        // a rejection the server explained shows its words, one it didn't falls back to the endpoint's sentence
        assert_eq!(STATS.message(&rejected(500, "error occurred")), "error occurred");
        assert_eq!(STATS.message(&rejected(502, "")), "Unexpected statistics response.");
        assert_eq!(STATS.message(&conflict(3, "taken")), "taken");

        assert_eq!(SUBMISSION.message(&ApiError::Unauthorized), "Sign in to view this submission.");
        assert_eq!(SUBMISSION.message(&ApiError::NotFound), "This submission is unavailable.");
        assert_eq!(SUBMISSION.message(&rejected(500, "")), "Unexpected submission response.");

        assert_eq!(VALUES.message(&ApiError::Unauthorized), "Sign in to load addon values.");
        assert_eq!(VALUES.message(&ApiError::NotFound), "This source is unavailable.");
        assert_eq!(VALUES.message(&ApiError::Decode(String::new())), "Unexpected values response.");

        assert_eq!(SAVE.message(&ApiError::Unauthorized), "Sign in to submit an addon.");
        assert_eq!(SAVE.message(&ApiError::NotFound), "This source is unavailable.");
        assert_eq!(SAVE.message(&rejected(400, "invalid json")), "invalid json");
        assert_eq!(SAVE.message(&ApiError::Network(String::new())), "Unexpected save response.");

        assert_eq!(WITHDRAW.message(&ApiError::Unauthorized), "Sign in to withdraw a submission.");
        assert_eq!(WITHDRAW.message(&ApiError::NotFound), "This source is unavailable.");
        assert_eq!(WITHDRAW.message(&ApiError::Network(String::new())), "Unexpected withdraw response.");

        assert_eq!(SOURCES.message(&ApiError::Unauthorized), "Sign in to view your addons.");
        assert_eq!(SOURCES.message(&ApiError::Decode(String::new())), "Unexpected sources response.");
        // the sources and the schema have no 'unavailable' sentence: a missing record reads as an unexpected reply
        assert_eq!(SOURCES.message(&ApiError::NotFound), "Unexpected sources response.");
        assert_eq!(SCHEMA.message(&ApiError::Unauthorized), "Sign in to edit addon declarations.");
        assert_eq!(SCHEMA.message(&ApiError::NotFound), "Unexpected schema response.");
        assert_eq!(UNSUPPORTED_SCHEMA, "This app needs an update to edit addon declarations.");
    }

    #[test]
    fn malformed_ids_read_as_missing_records() {
        assert_eq!(
            malformed_is_not_found(rejected(400, INVALID_VALUES_QUERY), INVALID_VALUES_QUERY),
            ApiError::NotFound
        );
        assert_eq!(malformed_is_not_found(rejected(400, INVALID_ADDON_UUID), INVALID_ADDON_UUID), ApiError::NotFound);
        assert_eq!(
            malformed_is_not_found(rejected(400, INVALID_SUBMISSION_ID), INVALID_SUBMISSION_ID),
            ApiError::NotFound
        );
        // another 400, another endpoint's words or another status are not a missing record
        assert_eq!(
            malformed_is_not_found(rejected(400, "invalid json"), INVALID_SUBMISSION_ID),
            rejected(400, "invalid json")
        );
        assert_eq!(
            malformed_is_not_found(rejected(400, INVALID_ADDON_UUID), INVALID_SUBMISSION_ID),
            rejected(400, INVALID_ADDON_UUID)
        );
        assert_eq!(
            malformed_is_not_found(rejected(500, INVALID_SUBMISSION_ID), INVALID_SUBMISSION_ID),
            rejected(500, INVALID_SUBMISSION_ID)
        );
        assert_eq!(malformed_is_not_found(ApiError::Unauthorized, INVALID_SUBMISSION_ID), ApiError::Unauthorized);
    }

    #[test]
    fn conflicts_name_the_submission_state() {
        assert_eq!(
            SubmissionConflict::of(&conflict(4, "submission already open")),
            Some(SubmissionConflict::AlreadyOpen(4))
        );
        assert_eq!(SubmissionConflict::of(&conflict(9, "submission not open")), Some(SubmissionConflict::NotOpen(9)));
        assert_eq!(SubmissionConflict::of(&conflict(1, "something else")), None);
        assert_eq!(SubmissionConflict::of(&rejected(409, "submission not open")), None);
        assert_eq!(SubmissionConflict::of(&ApiError::NotFound), None);
    }

    #[test]
    fn validate_results() {
        assert_eq!(validate_outcome(Ok(())), ValidateResult::Valid);
        let errors = vec![
            field_error("name", "too short"),
            field_error("name", "bad chars"),
            field_error("icon", "bad icon"),
            field_error("nonsense", "other thing"),
        ];
        let ValidateResult::Invalid { fields, other } = validate_outcome(Err(ApiError::Validation(errors))) else {
            panic!()
        };
        assert_eq!(fields["name"], vec!["too short", "bad chars"]);
        assert_eq!(fields["icon"], vec!["bad icon"]);
        assert_eq!(other, vec!["other thing"]);
        assert_eq!(validate_outcome(Err(conflict(9, "submission not open"))), ValidateResult::NotOpen(9));
        // only a closed submission is 'not open': any other conflict, or a missing record, is a failed check
        assert_eq!(
            validate_outcome(Err(conflict(9, "submission already open"))),
            ValidateResult::Failed(conflict(9, "submission already open"))
        );
        assert_eq!(validate_outcome(Err(ApiError::NotFound)), ValidateResult::Failed(ApiError::NotFound));
        assert_eq!(validate_outcome(Err(ApiError::Unauthorized)), ValidateResult::Failed(ApiError::Unauthorized));
    }

    #[test]
    fn save_results() {
        assert_eq!(save_outcome(Ok(12)), SubmitAddonResult::Submitted(12));
        assert_eq!(save_outcome(Err(conflict(4, "submission already open"))), SubmitAddonResult::AlreadyOpen(4));
        assert_eq!(save_outcome(Err(conflict(3, "submission not open"))), SubmitAddonResult::NotOpen(3));
        let invalid = save_outcome(Err(ApiError::Validation(vec![field_error("tags", "pick one")])));
        let SubmitAddonResult::Invalid { fields, other } = invalid else { panic!() };
        assert_eq!(fields["tags"], vec!["pick one"]);
        assert!(other.is_empty());
        assert_eq!(
            save_outcome(Err(ApiError::Unauthorized)),
            SubmitAddonResult::Error("Sign in to submit an addon.".into())
        );
        assert_eq!(
            save_outcome(Err(ApiError::NotFound)),
            SubmitAddonResult::Error("This source is unavailable.".into())
        );
        assert_eq!(save_outcome(Err(rejected(500, ""))), SubmitAddonResult::Error("Unexpected save response.".into()));
        assert_eq!(save_outcome(Err(rejected(400, "invalid addon"))), SubmitAddonResult::Error("invalid addon".into()));
        // a conflict about something else is shown as the server said it
        assert_eq!(save_outcome(Err(conflict(1, "busy"))), SubmitAddonResult::Error("busy".into()));
    }

    #[test]
    fn withdraw_results() {
        assert_eq!(withdraw_outcome(Ok(())), WithdrawResult::Withdrawn);
        assert_eq!(withdraw_outcome(Err(conflict(5, "submission not open"))), WithdrawResult::NotOpen(5));
        assert_eq!(
            withdraw_outcome(Err(ApiError::NotFound)),
            WithdrawResult::NotFound("This source is unavailable.".into())
        );
        assert_eq!(
            withdraw_outcome(Err(ApiError::Unauthorized)),
            WithdrawResult::Error("Sign in to withdraw a submission.".into())
        );
        assert_eq!(
            withdraw_outcome(Err(ApiError::Network("down".into()))),
            WithdrawResult::Error("Unexpected withdraw response.".into())
        );
        assert_eq!(
            withdraw_outcome(Err(conflict(5, "submission already open"))),
            WithdrawResult::Error("submission already open".into())
        );
    }

    #[test]
    fn upload_failures() {
        let refused = ApiError::Validation(vec![field_error("icon", "Icon must be square (1:1).")]);
        assert_eq!(UploadError::Api(refused).message(), "Icon must be square (1:1).");
        let other_field = ApiError::Validation(vec![field_error("name", "taken")]);
        assert_eq!(UploadError::Api(other_field).message(), "Icon upload failed.");
        assert_eq!(UploadError::Api(rejected(429, "icon upload limit reached")).message(), "Icon upload failed.");
        assert_eq!(UploadError::Api(ApiError::Unauthorized).message(), "Icon upload failed.");
        assert_eq!(UploadError::Api(ApiError::Decode("malformed icon upload".into())).message(), "Icon upload failed.");
        let unreadable = UploadError::from(std::io::Error::from(std::io::ErrorKind::NotFound));
        assert_eq!(unreadable.message(), "Icon upload failed.");
    }
}
