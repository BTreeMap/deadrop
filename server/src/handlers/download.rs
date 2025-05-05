use actix_web::{HttpRequest, HttpResponse};

pub async fn handle_download(_req: HttpRequest) -> HttpResponse {
    // Call the actual implementation here (to be implemented in another module)
    HttpResponse::Ok().body("download handler stub")
}
