use age::Decryptor;
use age::Encryptor;
use age::Identity;
use age::secrecy::ExposeSecret;
use age::x25519;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use clap::{Parser, Subcommand};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use url::Url;

const DEFAULT_ENDPOINT: &str = "https://deadrop.joefang.org";

#[derive(Parser)]
#[command(name = "deadrop")]
#[command(about = "Native Rust client for deadrop", long_about = None)]
struct Cli {
    #[arg(short, long, global = true, env = "ENDPOINT", default_value = DEFAULT_ENDPOINT)]
    endpoint: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Keygen {
        #[arg(short, long, default_value = "id_x25519")]
        output: PathBuf,
    },
    Send {
        #[arg(short, long)]
        pubkey: String,
        #[arg(short, long, conflicts_with = "file")]
        message: Option<String>,
        #[arg(short, long, conflicts_with = "message")]
        file: Option<PathBuf>,
    },
    Receive {
        #[arg(short, long)]
        identity: PathBuf,
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },
}

#[derive(Debug)]
enum CliError {
    InvalidInput(String),
    Io(std::io::Error),
    Http(reqwest::Error),
    HttpStatus { status: StatusCode, body: String },
    Json(serde_json::Error),
    Crypto(String),
    Url(url::ParseError),
}

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(msg) => write!(f, "{msg}"),
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Http(err) => write!(f, "Network request failed: {err}"),
            Self::HttpStatus { status, body } => {
                write!(f, "Server returned HTTP {status}: {body}")
            }
            Self::Json(err) => write!(f, "Invalid JSON payload: {err}"),
            Self::Crypto(msg) => write!(f, "Cryptography error: {msg}"),
            Self::Url(err) => write!(f, "Invalid endpoint URL: {err}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<reqwest::Error> for CliError {
    fn from(value: reqwest::Error) -> Self {
        Self::Http(value)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<url::ParseError> for CliError {
    fn from(value: url::ParseError) -> Self {
        Self::Url(value)
    }
}

#[derive(Serialize)]
struct ChallengeRequest {
    pubkey: String,
    scope: String,
}

#[derive(Deserialize)]
struct ChallengeResponse {
    ciphertext: String,
}

#[derive(Deserialize)]
struct RetrieveResponse {
    items: Vec<String>,
    next_cursor: Option<String>,
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), CliError> {
    let cli = Cli::parse();
    let endpoint = normalize_endpoint(&cli.endpoint)?;
    let http = reqwest::Client::builder().build()?;

    match cli.command {
        Commands::Keygen { output } => keygen(&output)?,
        Commands::Send {
            pubkey,
            message,
            file,
        } => {
            let plaintext = get_send_payload(message, file)?;
            send(&http, &endpoint, pubkey, plaintext).await?;
        }
        Commands::Receive { identity, output } => {
            receive(&http, &endpoint, &identity, &output).await?;
        }
    }

    Ok(())
}

fn normalize_endpoint(endpoint: &str) -> Result<Url, CliError> {
    let mut base = Url::parse(endpoint)?;
    if !base.path().ends_with('/') {
        let path = format!("{}/", base.path());
        base.set_path(&path);
    }
    Ok(base)
}

fn keygen(output: &Path) -> Result<(), CliError> {
    let identity = x25519::Identity::generate();
    let recipient = identity.to_public();

    let private_contents = format!("{}\n", identity.to_string().expose_secret());
    fs::write(output, private_contents)?;

    let pub_path = output.with_extension("pub");
    let public_contents = format!("{}\n", recipient);
    fs::write(&pub_path, public_contents)?;

    println!("Private key written to {}", output.display());
    println!("Public key written to {}", pub_path.display());
    Ok(())
}

fn get_send_payload(message: Option<String>, file: Option<PathBuf>) -> Result<Vec<u8>, CliError> {
    match (message, file) {
        (Some(m), None) => Ok(m.into_bytes()),
        (None, Some(path)) => fs::read(path).map_err(CliError::from),
        (None, None) => Err(CliError::InvalidInput(
            "Either --message or --file must be provided".to_string(),
        )),
        (Some(_), Some(_)) => Err(CliError::InvalidInput(
            "Use either --message or --file, not both".to_string(),
        )),
    }
}

fn parse_pubkey(pubkey_input: &str) -> Result<x25519::Recipient, CliError> {
    let content = if Path::new(pubkey_input).exists() {
        fs::read_to_string(pubkey_input)?
    } else {
        pubkey_input.to_string()
    };

    let trimmed = content.trim();
    trimmed.parse::<x25519::Recipient>().map_err(|err| {
        CliError::InvalidInput(format!(
            "Invalid age recipient public key '{}': {err}",
            trimmed
        ))
    })
}

fn read_identity(identity_path: &Path) -> Result<x25519::Identity, CliError> {
    let content = fs::read_to_string(identity_path)
        .map_err(|e| CliError::InvalidInput(format!("Cannot read identity file: {e}")))?;

    let line = content
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .ok_or_else(|| {
            CliError::InvalidInput("Identity file does not contain an age secret key".to_string())
        })?;

    line.parse::<x25519::Identity>().map_err(|e| {
        CliError::InvalidInput(format!(
            "Invalid age identity file '{}': {e}",
            identity_path.display()
        ))
    })
}

fn encrypt_for_recipient(
    recipient: &x25519::Recipient,
    plaintext: &[u8],
) -> Result<Vec<u8>, CliError> {
    let encryptor =
        Encryptor::with_recipients(std::iter::once(recipient as &dyn age::Recipient))
            .map_err(|e| CliError::Crypto(format!("failed to initialize encryptor: {e}")))?;

    let mut encrypted = Vec::new();
    let mut writer = encryptor
        .wrap_output(&mut encrypted)
        .map_err(|e| CliError::Crypto(format!("failed to begin encryption: {e}")))?;

    writer
        .write_all(plaintext)
        .map_err(|e| CliError::Crypto(format!("failed to write plaintext: {e}")))?;

    writer
        .finish()
        .map_err(|e| CliError::Crypto(format!("failed to finalize encryption: {e}")))?;

    Ok(encrypted)
}

fn decrypt_with_identity(
    identity: &x25519::Identity,
    ciphertext: &[u8],
) -> Result<Vec<u8>, CliError> {
    let decryptor = Decryptor::new(Cursor::new(ciphertext))
        .map_err(|e| CliError::Crypto(format!("invalid age ciphertext: {e}")))?;

    let mut reader = decryptor
        .decrypt(std::iter::once(identity as &dyn Identity))
        .map_err(|e| {
            CliError::InvalidInput(format!("Cannot decrypt with provided identity: {e}"))
        })?;

    let mut out = Vec::new();
    reader
        .read_to_end(&mut out)
        .map_err(|e| CliError::Crypto(format!("failed to read decrypted content: {e}")))?;
    Ok(out)
}

async fn send(
    http: &reqwest::Client,
    endpoint: &Url,
    pubkey_input: String,
    plaintext: Vec<u8>,
) -> Result<(), CliError> {
    let recipient = parse_pubkey(&pubkey_input)?;
    let pubkey = recipient.to_string();
    let encrypted = encrypt_for_recipient(&recipient, &plaintext)?;

    let upload_url = endpoint.join("upload")?;
    let response = http
        .post(upload_url)
        .header("X-PubKey", pubkey)
        .body(encrypted)
        .send()
        .await?;

    if response.status() != StatusCode::CREATED {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(CliError::HttpStatus { status, body });
    }

    println!("Upload successful.");
    Ok(())
}

async fn authenticate(
    http: &reqwest::Client,
    endpoint: &Url,
    identity: &x25519::Identity,
) -> Result<String, CliError> {
    let challenge_url = endpoint.join("challenge")?;
    let payload = ChallengeRequest {
        pubkey: identity.to_public().to_string(),
        scope: "retrieve".to_string(),
    };

    let response = http.post(challenge_url).json(&payload).send().await?;
    if response.status() != StatusCode::OK {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(CliError::HttpStatus { status, body });
    }

    let body: ChallengeResponse = response.json().await?;
    let challenge_ciphertext = BASE64
        .decode(body.ciphertext.as_bytes())
        .map_err(|e| CliError::Crypto(format!("challenge ciphertext is not valid base64: {e}")))?;

    let jwt = decrypt_with_identity(identity, &challenge_ciphertext)?;
    String::from_utf8(jwt)
        .map_err(|e| CliError::Crypto(format!("challenge JWT is not valid UTF-8: {e}")))
}

async fn receive(
    http: &reqwest::Client,
    endpoint: &Url,
    identity_path: &Path,
    output_dir: &Path,
) -> Result<(), CliError> {
    let identity = read_identity(identity_path)?;
    fs::create_dir_all(output_dir)?;

    let jwt = authenticate(http, endpoint, &identity).await?;

    let mut cursor: Option<String> = None;
    let mut found_any = false;
    loop {
        let mut retrieve_url = endpoint.join("retrieve")?;
        if let Some(cursor_value) = &cursor {
            retrieve_url
                .query_pairs_mut()
                .append_pair("cursor", cursor_value);
        }

        let response = http.post(retrieve_url).bearer_auth(&jwt).send().await?;

        if response.status() != StatusCode::OK {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(CliError::HttpStatus { status, body });
        }

        let retrieve: RetrieveResponse = response.json().await?;

        if retrieve.items.is_empty() && cursor.is_none() {
            println!("No items found.");
            return Ok(());
        }

        for item_id in retrieve.items {
            found_any = true;
            let download_url = endpoint.join(&format!("download/{item_id}"))?;
            let response = http.get(download_url).bearer_auth(&jwt).send().await?;

            if response.status() != StatusCode::OK {
                eprintln!(
                    "Warning: failed to download item {item_id} (HTTP {}).",
                    response.status()
                );
                continue;
            }

            let ciphertext = response.bytes().await?.to_vec();
            let encrypted_path = output_dir.join(format!("item_{item_id}.age"));
            let decrypted_path = output_dir.join(format!("item_{item_id}.dec"));

            fs::write(&encrypted_path, &ciphertext)?;

            match decrypt_with_identity(&identity, &ciphertext) {
                Ok(plaintext) => {
                    fs::write(&decrypted_path, plaintext)?;
                    println!(
                        "Downloaded {} and decrypted to {}",
                        encrypted_path.display(),
                        decrypted_path.display()
                    );
                }
                Err(err) => {
                    eprintln!(
                        "Warning: failed to decrypt {}: {}",
                        encrypted_path.display(),
                        err
                    );
                }
            }
        }

        if let Some(next) = retrieve.next_cursor {
            cursor = Some(next);
        } else {
            break;
        }
    }

    if !found_any {
        println!("No items found.");
    }

    Ok(())
}
