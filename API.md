# Deadrop API Specification

This document details the API endpoints for the Deadrop service.

## Authentication

Authentication relies on a challenge-response mechanism using the client's X25519 keypair.

1. **Challenge Request**: The client requests a challenge from the `POST /challenge` endpoint, providing its public key (`X-PubKey` header or in body) and the desired scope (`retrieve` or `notify`).
2. **Challenge Issuance**: The server generates a short-lived JSON Web Token (JWT) containing the public key (`sub`), scope (`aud`), timestamps (`iat`, `exp`), and potentially other scope-specific data (e.g., `telegram` target for notifications). This JWT is then encrypted using `age` (X25519) with the client's public key. The server returns the resulting ciphertext in the JSON response body (`{ "ciphertext": "..." }`).
3. **Challenge Response**: The client decrypts the ciphertext using its private key to obtain the JWT.
4. **Authenticated Request**: The client makes requests to scope-protected endpoints (e.g., `/retrieve`, `/notify`, `/download`) by including the decrypted JWT in the standard `Authorization` header: `Authorization: Bearer <jwt>`.
5. **Verification**: The server verifies the JWT's signature, expiration (`exp`), and audience (`aud`) claim against the requested endpoint. The subject (`sub`) claim identifies the authenticated public key.

* `X-PubKey`: The client's public key (base64 encoded). Required for `/upload` and `/challenge`.

## Endpoints

### `POST /upload`

Uploads encrypted data associated with a public key.

* **Headers**:
  * `X-PubKey: <user X25519 pubkey (base64)>`
* **Body**: Raw binary ciphertext.
* **Response**:
  * `201 Created`: On successful upload.
  * `400 Bad Request`: If headers or body are invalid.

Server stores the binary blob associated with the provided public key and a timestamp.

### `POST /challenge`

Initiates the authentication process by requesting an encrypted challenge token.

* **Headers**:
  * `X-PubKey: <user X25519 pubkey (base64)>`
* **Body**: JSON object specifying the scope and any related data.

  ```json
  {
    "scope": "<retrieve|notify>",
    // Optional, only for 'notify' scope:
    "telegram": "<telegram_user_id_or_handle>"
  }
  ```

* **Response**:
  * `200 OK`
    * **Body**: JSON object containing the age-encrypted JWT.

      ```json
      {
        "ciphertext": "<age-encrypted JWT (base64)>"
      }
      ```

  * `400 Bad Request`: If `X-PubKey` header or body is missing or invalid.
  * `404 Not Found`: If the public key is not recognized (optional, depends on server logic).

### `POST /retrieve`

Retrieves a paginated list of available item IDs after successful authentication.

* **Headers**:
  * `Authorization: Bearer <signed JWT>` (Obtained from decrypting `/challenge` response)
* **Query Parameters**:
  * `cursor` (optional): The item ID (UUID string) to start pagination from (exclusive). If omitted, returns the first page.
* **Body**: Empty.
* **Response**:
  * `200 OK`: On successful authentication and verification.
    * **Body**: JSON object containing an array of item IDs and an optional `next_cursor` for pagination.

      ```json
      {
        "items": [
          "<item_id_1>",
          "<item_id_2>",
          "..."
        ],
        "next_cursor": "<item_id_N>" // Omitted if no more items
      }
      ```

    * Items are ordered by `created_at` (descending), then by `id` (descending). The number of items per page is fixed by the server configuration and cannot be changed by the client.
    * To fetch the next page, use the `next_cursor` value as the `cursor` query parameter in the next request. If `next_cursor` is absent, there are no more items.

  * `401 Unauthorized`: If the JWT is missing, invalid (signature, expiration, `aud` claim != `/retrieve`), or the `sub` key has no items.
  * `400 Bad Request`: If headers or cursor are malformed.

### `GET /download/{item_id}`

Downloads a specific item's ciphertext. Requires prior successful authentication via `/retrieve`.

* **Path Parameter**:
  * `item_id`: The unique identifier of the item to download.
* **Headers**:
  * `Authorization: Bearer <signed JWT>` (The same token used for `/retrieve` should be valid if within TTL)
* **Response**:
  * `200 OK`:
    * **Body**: Raw binary ciphertext of the requested item.
  * `401 Unauthorized`: If the JWT is missing, invalid, or expired.
  * `403 Forbidden`: If the JWT is valid but the `item_id` does not belong to the public key in the JWT `sub` claim.
  * `404 Not Found`: If the `item_id` is invalid.

### `POST /notify`

Registers a notification hook after successful authentication.

* **Headers**:
  * `Authorization: Bearer <signed JWT>` (Obtained from decrypting `/challenge` response for `notify` scope)
* **Body**: Empty. (Notification details are now embedded in the JWT from the `/challenge` step).
* **Response**:
  * `200 OK`: On successful registration.
  * `401 Unauthorized`: If the JWT is missing, invalid (signature, expiration, `aud` claim != `/notify`, missing `telegram` claim).
  * `400 Bad Request`: If headers are malformed.

## Security Considerations

* **Stateless Authentication**: The encrypted JWT issued by `/challenge` contains all necessary state (`sub`, `aud`, `exp`, `iat`, scope-specific data). Authenticated endpoints verify the presented `Authorization: Bearer` token.
* **Key Usage**: Single X25519 keypair used for identifying users (`X-PubKey` for challenge request) and proving ownership (by decrypting the challenge and using the resulting JWT).
* **Transport Security**: TLS is essential to protect headers (including `Authorization` and `X-PubKey`) and bodies in transit.
* **JWT Security**: Standard JWT practices apply: short expiration (`exp`), audience restriction (`aud`), secure signing algorithm (HS256 assumed here, but others possible), protection against replay (via `exp` and potentially `jti`). Encryption via `age` protects the token content until decrypted by the intended recipient.
* **Rate Limiting**: Implement rate limiting on all endpoints, especially `/challenge`, `/retrieve`, and `/notify`, to mitigate brute-force and denial-of-service attacks.
