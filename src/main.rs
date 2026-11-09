//! Demo web server: "MyApp" with TOTP enrollment and verification.
//! See docs/api-contract.md for the routes and JSON shapes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::State,
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use qrcode::{render::svg, QrCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use totp_poc::totp::{self, VerifyError};

const ISSUER: &str = "TOTP-POC";
const ADDR: &str = "127.0.0.1:3000";

/// What the server remembers per user. Lost on restart (in-memory POC).
struct User {
    secret: Vec<u8>,
    /// Time step of the last accepted code, used to reject replays.
    last_used_step: Option<u64>,
}

/// Shared state: `Arc` lets every request handler hold a reference to the same
/// map, `Mutex` makes sure only one handler changes it at a time.
type Users = Arc<Mutex<HashMap<String, User>>>;

#[derive(Deserialize)]
struct EnrollRequest {
    username: String,
}

#[derive(Serialize)]
struct EnrollResponse {
    username: String,
    secret: String,
    otpauth_uri: String,
    qr_svg: String,
}

#[derive(Deserialize)]
struct VerifyRequest {
    username: String,
    code: String,
}

#[derive(Serialize)]
struct VerifyResponse {
    ok: bool,
    message: &'static str,
}

#[tokio::main]
async fn main() {
    let users: Users = Arc::new(Mutex::new(HashMap::new()));

    let app = Router::new()
        .route("/", get(|| async { Html(include_str!("../static/index.html")) }))
        .route(
            "/authenticator",
            get(|| async { Html(include_str!("../static/authenticator.html")) }),
        )
        .route("/api/enroll", post(enroll))
        .route("/api/verify", post(verify))
        .with_state(users);

    let listener = tokio::net::TcpListener::bind(ADDR)
        .await
        .unwrap_or_else(|e| panic!("could not bind {ADDR}: {e}"));
    println!("MyApp:            http://localhost:3000");
    println!("My Authenticator: http://localhost:3000/authenticator");
    axum::serve(listener, app).await.expect("server error");
}

async fn enroll(
    State(users): State<Users>,
    Json(req): Json<EnrollRequest>,
) -> Result<Json<EnrollResponse>, (StatusCode, Json<Value>)> {
    let username = req.username.trim().to_string();
    if username.is_empty() {
        return Err(bad_request("username must not be empty"));
    }

    let secret = totp::generate_secret();
    let secret_b32 = totp::to_base32(&secret);
    let otpauth_uri = totp::otpauth_uri(ISSUER, &username, &secret_b32);
    let qr_svg = QrCode::new(otpauth_uri.as_bytes())
        .map_err(|e| bad_request(&format!("could not build QR code: {e}")))?
        .render::<svg::Color>()
        .min_dimensions(220, 220)
        .build();

    users.lock().unwrap().insert(
        username.clone(),
        User { secret: secret.to_vec(), last_used_step: None },
    );

    Ok(Json(EnrollResponse { username, secret: secret_b32, otpauth_uri, qr_svg }))
}

async fn verify(State(users): State<Users>, Json(req): Json<VerifyRequest>) -> Json<VerifyResponse> {
    let mut users = users.lock().unwrap();
    let Some(user) = users.get_mut(req.username.trim()) else {
        return reply(false, "Unknown user");
    };

    match totp::verify(&user.secret, &req.code, unix_now(), user.last_used_step) {
        Ok(step) => {
            user.last_used_step = Some(step);
            reply(true, "Code accepted")
        }
        Err(VerifyError::InvalidCode) => reply(false, "Invalid code"),
        Err(VerifyError::Replayed) => reply(false, "Code already used"),
    }
}

fn reply(ok: bool, message: &'static str) -> Json<VerifyResponse> {
    Json(VerifyResponse { ok, message })
}

fn bad_request(msg: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": msg })))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before 1970")
        .as_secs()
}
