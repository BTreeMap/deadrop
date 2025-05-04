# deadrop (WIP Proposal)

**Status:** This document is currently a proposal/Work-in-Progress; no specific timeline for completion has been set.

Encrypted "dead‑drop" service allowing users to anonymously submit and retrieve data using a single X25519 keypair. Data is encrypted client-side and stored server-side under your public key. Retrieval and notification APIs perform a stateless proof-of-possession via a single challenge endpoint returning an encrypted JWT, which is then used with standard `Authorization: Bearer` headers.

---

## Features

* **Client (`deadrop.sh`)**

  * `send`: encrypt & upload text or files
  * `retrieve`: authenticate & download all ciphertexts for your key
  * `notify`: register Telegram push notifications
  * Fully stateless challenge using encrypted JWTs; no server-side session storage

* **Server (`deadrop.joefang.org`)**

  * `POST /upload`: store encrypted payload
  * `POST /challenge`: issue encrypted JWT challenge for a given scope (retrieve/notify)
  * `POST /retrieve`: verify JWT & return stored items
  * `POST /notify`: verify JWT & register Telegram hook

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

1. Client calls `POST /challenge` with `{ "pubkey": "<pub>", "scope": "retrieve" }` → returns `{ "ciphertext": "<age-encrypted JWT>" }`
2. Client decrypts ciphertext to get the JWT.
3. Client calls `POST /retrieve` with `Authorization: Bearer <jwt>` header.
4. Server verifies JWT (`sub`, `aud: "/retrieve"`, `exp`), then returns stored items as JSON list of base64 blobs.

Each item is saved and decrypted locally.

### `notify`

Register a Telegram hook:

```sh
deadrop.sh notify -i id_x25519 -t "@alice"  # or numeric user ID
```

1. Client calls `POST /challenge` with `{ "pubkey": "<pub>", "scope": "notify", "telegram": "<target>" }` → returns `{ "ciphertext": "<age-encrypted JWT>" }`
2. Client decrypts ciphertext to get the JWT.
3. Client calls `POST /notify` with `Authorization: Bearer <jwt>` header.
4. Server verifies JWT (`sub`, `aud: "/notify"`, `exp`, `telegram`), then registers the hook.

---

## Server API

All endpoints expect and return JSON unless otherwise specified. Client decrypts the JWT challenge received from `/challenge`.

### `POST /upload`

* **Headers**: `X-PubKey: <user X25519 pub>`
* **Body**: raw ciphertext
* **Response**: `201 Created` on success

Server stores each blob with its associated public key and timestamp.

### `POST /challenge`

* **Body**: `{ "pubkey": "<user X25519 pub>", "scope": "<retrieve|notify>", "telegram"?: "<telegram_target_if_notify>" }`
* **Response**: `{ "ciphertext": "<age-encrypted JWT>" }`

Server creates a JWT with:

```json
{
  "sub": "<pubkey>",
  "aud": "/<scope>", // e.g., "/retrieve" or "/notify"
  "iat": <now>,
  "exp": <now + 300>,
  "telegram"?: "<telegram_target_if_notify>" // Included only for notify scope
}
```

Then signs (HS256) and encrypts via age for the user’s `pubkey`.

### `POST /retrieve`

* **Headers**: `Authorization: Bearer <signed JWT>`
* **Response**: `{ "items": [ "<base64-cipher1>", ... ] }`

Server verifies JWT signature, `aud: "/retrieve"`, `exp`, matches `sub` to an existing user with data, then returns stored blobs associated with the `sub` (pubkey).

### `POST /notify`

* **Headers**: `Authorization: Bearer <signed JWT>`
* **Response**: `200 OK`

Server verifies JWT signature, `aud: "/notify"`, `exp`, and the presence of the `telegram` claim. It then registers the Telegram ID from the claim for push notifications on future uploads associated with the `sub` (pubkey).

---

## Security Considerations

* **Single keypair**: only X25519 used for both encryption and proof-of-possession.
* **Stateless Authentication**: Encrypted JWT carries necessary claims (`iat`/`exp`/`sub`/`aud`/`telegram`); server verifies the token presented in the `Authorization` header. No server-side session state needed after issuing the challenge.
* **Short TTL** (default 5 minutes) on JWT prevents replay attacks.
* **TLS** protects headers (including `Authorization`) and bodies in transit.
* **Scope Claim**: The `aud` (audience) claim in the JWT ensures a token issued for one purpose (e.g., `retrieve`) cannot be used for another (e.g., `notify`).

---

## License

MIT © Joe Fang
