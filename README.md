# deadrop (WIP Proposal)

**Status:** This document is currently a proposal/Work-in-Progress; no specific timeline for completion has been set.

Encrypted "dead‑drop" service allowing users to anonymously submit and retrieve data using a single X25519 keypair. Data is encrypted client-side and stored server-side under your public key. Retrieval and notification APIs perform a stateless proof-of-possession via short‑lived, encrypted JWT challenges.

---

## Features

* **Client (`deadrop.sh`)**

  * `send`: encrypt & upload text or files
  * `retrieve`: authenticate & download all ciphertexts for your key
  * `notify`: register Telegram push notifications
  * Fully stateless challenge using encrypted JWTs; no server-side session storage

* **Server (`deadrop.joefang.org`)**

  * `POST /upload`: store encrypted payload
  * `POST /retrieve`: issue encrypted JWT challenge
  * `POST /retrieve/confirm`: verify JWT & return stored items
  * `POST /notify`: issue encrypted JWT challenge (embed `telegram` claim)
  * `POST /notify/confirm`: verify JWT & register Telegram hook

---

## Prerequisites

* **Client**

  * `bash` (≥ 4.4)
  * [`age`](https://github.com/FiloSottile/age) (X25519 encryption)
  * [`curl`](https://curl.se)
  * [`jq`](https://stedolan.github.io/jq)

* **Server**

  * HTTP server (e.g. NGINX) with TLS
  * Application runtime (e.g. Go/Python/Node)
  * Database (e.g. PostgreSQL) for storing ciphertexts and notification registrations
  * JWT library supporting HS256

---

## Installation

Clone the repo and make the client script executable:

```sh
git clone https://github.com/joefang/deadrop.git
cd deadrop
chmod +x deadrop.sh
```

Server code is in `/server`. See its own README for deployment instructions.

---

## Client Usage (`deadrop.sh`)

### Common flags

* `-k, --pubkey <file|string>`
  X25519 public key (file path or raw string)

* `-i, --identity <file>`
  X25519 private key for decryption/authentication

* `-e, --endpoint <URL>`
  Override default server URL (env var `ENDPOINT` also respected)

### `send`

Encrypt and upload data:

```sh
# upload a string
deadrop.sh send -k id_x25519.pub -m "hello world"

# upload a file
deadrop.sh send -k id_x25519.pub -f /path/to/secret.txt
```

Server endpoint: `POST /upload`
Headers: `X-PubKey: <pubkey>`
Body: binary ciphertext

### `retrieve`

Authenticate via encrypted JWT and download items:

```sh
deadrop.sh retrieve -i id_x25519 -o ./downloads
```

1. `POST /retrieve` with `{ "pubkey": "<pub>" }` → returns base64 `ciphertext`
2. Client decrypts to JWT, then `POST /retrieve/confirm` with `{ "token": "<jwt>" }`
3. Server verifies JWT `sub`, `aud: "/retrieve"`, `exp`, then returns stored items as JSON list of base64 blobs

Each item is saved and decrypted locally.

### `notify`

Register a Telegram hook:

```sh
deadrop.sh notify -i id_x25519 -t "@alice"  # or numeric user ID
```

Same two-step flow against `/notify` and `/notify/confirm`. JWT payload includes `telegram` claim.

---

## Server API

All endpoints expect and return JSON. Client encrypts all JWT challenges via age.

### `POST /upload`

* **Headers**: `X-PubKey: <user X25519 pub>`
* **Body**: raw ciphertext
* **Response**: `200 OK` on success

Server stores each blob with its associated public key and timestamp.

### `POST /retrieve`

* **Body**: `{ "pubkey": "<user X25519 pub>" }`
* **Response**: `{ "ciphertext": "<age-encrypted JWT>" }`

Server creates a JWT with:

```json
{
  "sub": "<pubkey>",
  "aud": "/retrieve",
  "iat": <now>,
  "exp": <now + 300>
}
```

Then signs and encrypts via age for the user’s `pubkey`.

### `POST /retrieve/confirm`

* **Body**: `{ "token": "<signed JWT>" }`
* **Response**: `{ "items": [ "<base64-cipher1>", ... ] }`

Server verifies signature, `aud`, `exp`, matches `sub`, then returns stored blobs.

### `POST /notify`

* **Body**: `{ "pubkey": "<user X25519 pub>" }`
* **Response**: `{ "ciphertext": "<age-encrypted JWT>" }`

JWT payload also includes `telegram` once confirmed.

### `POST /notify/confirm`

* **Body**: `{ "token": "<signed JWT>" }`
* **Response**: `200 OK`

Server verifies and registers the Telegram ID for push notifications on future uploads.

---

## Security Considerations

* **Single keypair**: only X25519 used for both encryption and proof-of-possession
* **Stateless**: encrypted JWT carries `iat`/`exp`/`sub`/`aud`; no server-side cache
* **Short TTL** (default 5 minutes) prevents replay attacks
* **TLS** protects tokens in transit

---

## License

MIT © Joe Fang
