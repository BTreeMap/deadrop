#!/bin/bash
# deadrop.sh - Client for interacting with the deadrop service

set -euo pipefail

# --- Configuration ---
DEFAULT_ENDPOINT="https://deadrop.joefang.org" # Replace with actual default endpoint if different
INSTALL_DEPS_URL="https://raw.githubusercontent.com/joefang/deadrop/main/install-deps.sh" # Assumed URL for dependency installer

# --- Global Variables ---
ENDPOINT="${ENDPOINT:-$DEFAULT_ENDPOINT}"
PUBKEY=""
IDENTITY_FILE=""
MESSAGE=""
FILE_PATH=""
OUTPUT_DIR="."
TELEGRAM_TARGET=""

# --- Helper Functions ---

# Function to print usage instructions
usage() {
  cat << EOF
Usage: deadrop.sh <command> [options]

Commands:
  send      Encrypt and upload data.
  retrieve  Authenticate and download items.
  notify    Register a Telegram notification hook.

Common Options:
  -e, --endpoint <URL>    Server endpoint URL (default: $DEFAULT_ENDPOINT, env: ENDPOINT)
  -h, --help              Show this help message

'send' Options:
  -k, --pubkey <file|key> Public key (file path or raw string) for encryption. (Required)
  -m, --message <string>  Message string to encrypt and upload. (Use either -m or -f)
  -f, --file <path>       File path to encrypt and upload. (Use either -m or -f)

'retrieve' Options:
  -i, --identity <file>   Private key file (X25519) for decryption/authentication. (Required)
  -o, --output <dir>      Directory to save downloaded items (default: current directory).

'notify' Options:
  -i, --identity <file>   Private key file (X25519) for authentication. (Required)
  -t, --telegram <target> Telegram user ID or @handle for notifications. (Required)
EOF
  exit 1
}

# Function to check for required dependencies
check_deps() {
  local missing_deps=()
  command -v age >/dev/null 2>&1 || missing_deps+=("age")
  command -v curl >/dev/null 2>&1 || missing_deps+=("curl")
  command -v jq >/dev/null 2>&1 || missing_deps+=("jq")
  command -v base64 >/dev/null 2>&1 || missing_deps+=("base64 (coreutils)") # Needed for JWT/API interactions

  if [ ${#missing_deps[@]} -gt 0 ]; then
    echo "Error: Missing required dependencies: ${missing_deps[*]}" >&2
    echo "Attempting to download and run the dependency installer..." >&2
    if command -v curl >/dev/null 2>&1; then
      curl -sSL "$INSTALL_DEPS_URL" | bash -s -- || {
        echo "Error: Failed to download or run the dependency installer from $INSTALL_DEPS_URL" >&2
        echo "Please install the missing dependencies manually." >&2
        exit 1
      }
      # Re-check after attempting install
      check_deps
    else
      echo "Error: 'curl' is required to download the dependency installer." >&2
      echo "Please install curl and the other missing dependencies (${missing_deps[*]}) manually." >&2
      exit 1
    fi
  fi
  # Check for age-keygen specifically
  command -v age-keygen >/dev/null 2>&1 || {
      echo "Error: 'age-keygen' (part of age) not found. Please ensure 'age' is correctly installed." >&2
      exit 1
  }
}

# Function to handle errors
error_exit() {
  echo "Error: $1" >&2
  exit 1
}

# Function to get public key string (handles file path or raw key)
get_pubkey_str() {
  local key_input="$1"
  if [ -f "$key_input" ]; then
    cat "$key_input"
  else
    echo "$key_input"
  fi
}

# Function to handle 'send' command
send_data() {
  # 1. Validate flags
  if [ -z "$PUBKEY" ]; then
    error_exit "Public key (-k) is required for send."
  fi
  if [ -z "$MESSAGE" ] && [ -z "$FILE_PATH" ]; then
    error_exit "Either a message (-m) or a file (-f) is required for send."
  fi
  if [ -n "$MESSAGE" ] && [ -n "$FILE_PATH" ]; then
    error_exit "Use either -m or -f for send, not both."
  fi

  # 2. Determine public key string
  local pubkey_str
  pubkey_str=$(get_pubkey_str "$PUBKEY") || error_exit "Failed to read public key from '$PUBKEY'."
  if [ -z "$pubkey_str" ]; then
      error_exit "Public key cannot be empty."
  fi
  local pubkey_b64
  pubkey_b64=$(echo -n "$pubkey_str" | base64) || error_exit "Failed to base64 encode public key."

  # 3. Prepare and encrypt input data
  local encrypted_data
  echo "Encrypting data..." >&2
  if [ -n "$MESSAGE" ]; then
    encrypted_data=$(echo -n "$MESSAGE" | age -r "$pubkey_str" -a) || error_exit "Encryption failed."
  elif [ -n "$FILE_PATH" ]; then
    if [ ! -f "$FILE_PATH" ]; then
      error_exit "File not found: $FILE_PATH"
    fi
    encrypted_data=$(age -r "$pubkey_str" -a "$FILE_PATH") || error_exit "Encryption failed."
  fi

  # 5. Upload using curl
  echo "Uploading data..." >&2
  local upload_status
  upload_status=$(curl -s -w "%{http_code}" -o /dev/null -X POST \
    -H "X-PubKey: $pubkey_b64" \
    --data-binary "$encrypted_data" \
    "$ENDPOINT/upload")

  if [ "$upload_status" -eq 201 ]; then
    echo "Upload successful." >&2
  else
    error_exit "Upload failed. Server responded with HTTP status $upload_status."
  fi
}

# Function to perform challenge-response authentication
authenticate() {
  local scope="$1"
  local extra_payload="$2" # Optional JSON string like ',"telegram":"target"'

  if [ -z "$IDENTITY_FILE" ]; then
    error_exit "Identity file (-i) is required for $scope."
  fi
  if [ ! -f "$IDENTITY_FILE" ]; then
    error_exit "Identity file not found: $IDENTITY_FILE"
  fi

  echo "Authenticating for scope '$scope'..." >&2

  # 2. Extract pubkey from identity
  local pubkey_str
  pubkey_str=$(age-keygen -y "$IDENTITY_FILE") || error_exit "Failed to extract public key from identity file."
  local pubkey_b64
  pubkey_b64=$(echo -n "$pubkey_str" | base64)

  # 3. Request challenge
  local challenge_payload="{\"scope\": \"$scope\"$extra_payload}"
  local challenge_response
  challenge_response=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -H "X-PubKey: $pubkey_b64" \
    -d "$challenge_payload" \
    "$ENDPOINT/challenge") || error_exit "Challenge request failed."

  # 4. Extract ciphertext from JSON response
  local ciphertext_b64
  ciphertext_b64=$(echo "$challenge_response" | jq -r '.ciphertext')
  if [ -z "$ciphertext_b64" ] || [ "$ciphertext_b64" == "null" ]; then
    error_exit "Failed to get ciphertext from challenge response: $challenge_response"
  fi

  # 5. Decode base64 ciphertext
  local decoded_ciphertext
  # Use platform-independent base64 decode
  if command -v base64 >/dev/null && base64 --version 2>/dev/null | grep -q GNU; then
    decoded_ciphertext=$(echo "$ciphertext_b64" | base64 -d)
  elif command -v base64 >/dev/null; then # macOS base64
    decoded_ciphertext=$(echo "$ciphertext_b64" | base64 -D)
  else
    error_exit "base64 command not found or unsupported."
  fi

  # 6. Decrypt JWT
  local jwt
  jwt=$(echo -n "$decoded_ciphertext" | age -d -i "$IDENTITY_FILE" 2>/dev/null) || error_exit "Failed to decrypt challenge JWT. Check identity file and server response."

  if [ -z "$jwt" ]; then
      error_exit "Decrypted JWT is empty. Authentication failed."
  fi

  echo "Authentication successful." >&2
  echo "$jwt" # Return JWT
}

# Function to handle 'retrieve' command
retrieve_data() {
  # 1. Validate flags
  if [ -z "$IDENTITY_FILE" ]; then
    error_exit "Identity file (-i) is required for retrieve."
  fi
  mkdir -p "$OUTPUT_DIR" || error_exit "Failed to create output directory: $OUTPUT_DIR"

  # Authenticate to get JWT
  local jwt
  jwt=$(authenticate "retrieve" "") || exit 1

  # 7. Request retrieval of item IDs
  echo "Retrieving item list..." >&2
  local retrieve_response
  retrieve_response=$(curl -s -w "\n%{http_code}" -X POST \
    -H "Authorization: Bearer $jwt" \
    "$ENDPOINT/retrieve")

  local retrieve_status=$(echo "$retrieve_response" | tail -n1)
  local retrieve_body=$(echo "$retrieve_response" | sed '$d')

  if [ "$retrieve_status" -ne 200 ]; then
    error_exit "Failed to retrieve item list. Server responded with HTTP status $retrieve_status. Body: $retrieve_body"
  fi

  # 8. Parse JSON response for item IDs
  local item_ids
  item_ids=$(echo "$retrieve_body" | jq -r '.[]')

  if [ -z "$item_ids" ]; then
    echo "No items found to retrieve." >&2
    exit 0
  fi

  echo "Found items:" >&2
  echo "$item_ids" >&2

  # 9. Loop through items:
  local item_count=0
  local download_count=0
  local decrypt_count=0
  for item_id in $item_ids; do
    item_count=$((item_count + 1))
    echo "Downloading item $item_id..." >&2
    local download_url="$ENDPOINT/download/$item_id"
    local output_enc_file="$OUTPUT_DIR/item_${item_id}.age"
    local output_dec_file="$OUTPUT_DIR/item_${item_id}.dec"

    # Download item ciphertext
    local download_status
    download_status=$(curl -s -w "%{http_code}" -o "$output_enc_file" -X GET \
      -H "Authorization: Bearer $jwt" \
      "$download_url")

    if [ "$download_status" -eq 200 ]; then
      echo "Downloaded to $output_enc_file" >&2
      download_count=$((download_count + 1))

      # Decrypt item
      echo "Decrypting $output_enc_file..." >&2
      if age -d -i "$IDENTITY_FILE" -o "$output_dec_file" "$output_enc_file"; then
        echo "Decrypted to $output_dec_file" >&2
        decrypt_count=$((decrypt_count + 1))
        # Optional: remove encrypted file after successful decryption
        # rm "$output_enc_file"
      else
        echo "Warning: Failed to decrypt $output_enc_file. Check identity file." >&2
        # Keep the encrypted file for manual inspection
      fi
    else
      echo "Warning: Failed to download item $item_id. Server responded with HTTP status $download_status." >&2
      rm -f "$output_enc_file" # Clean up empty file on failure
    fi
  done

  echo "Retrieve summary: Found $item_count items, downloaded $download_count, decrypted $decrypt_count." >&2
}

# Function to handle 'notify' command
notify_hook() {
  # 1. Validate flags
  if [ -z "$IDENTITY_FILE" ]; then
    error_exit "Identity file (-i) is required for notify."
  fi
  if [ -z "$TELEGRAM_TARGET" ]; then
    error_exit "Telegram target (-t) is required for notify."
  fi

  # Authenticate to get JWT
  local telegram_json_part
  # Escape potential special characters in telegram target for JSON
  local escaped_telegram
  escaped_telegram=$(echo "$TELEGRAM_TARGET" | jq -R -s '.')
  telegram_json_part=",\"telegram\": $escaped_telegram"
  local jwt
  jwt=$(authenticate "notify" "$telegram_json_part") || exit 1

  # 7. Request notification registration
  echo "Registering notification hook for $TELEGRAM_TARGET..." >&2
  local notify_status
  notify_status=$(curl -s -w "%{http_code}" -o /dev/null -X POST \
    -H "Authorization: Bearer $jwt" \
    "$ENDPOINT/notify")

  if [ "$notify_status" -eq 200 ]; then
    echo "Notification hook registered successfully." >&2
  else
    error_exit "Notification registration failed. Server responded with HTTP status $notify_status."
  fi
}

# --- Main Execution ---

# Check if any command is provided
if [ $# -eq 0 ]; then
  usage
fi

# Parse command
COMMAND="$1"
shift

# Parse options
while [ $# -gt 0 ]; do
  case "$1" in
    -e|--endpoint)
      ENDPOINT="$2"
      shift 2
      ;;
    -k|--pubkey)
      PUBKEY="$2"
      shift 2
      ;;
    -i|--identity)
      IDENTITY_FILE="$2"
      shift 2
      ;;
    -m|--message)
      MESSAGE="$2"
      shift 2
      ;;
    -f|--file)
      FILE_PATH="$2"
      shift 2
      ;;
    -o|--output)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    -t|--telegram)
      TELEGRAM_TARGET="$2"
      shift 2
      ;;
    -h|--help)
      usage
      ;;
    *)
      echo "Error: Unknown option: $1" >&2
      usage
      ;;
  esac
done

# Check dependencies *after* parsing args (in case help was requested)
check_deps

# Execute command
case "$COMMAND" in
  send)
    send_data
    ;;
  retrieve)
    retrieve_data
    ;;
  notify)
    notify_hook
    ;;
  *)
    echo "Error: Unknown command: $COMMAND" >&2
    usage
    ;;
esac

exit 0
