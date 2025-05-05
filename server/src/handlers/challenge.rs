use actix_web::{HttpRequest, HttpResponse};

pub async fn handle_challenge(_req: HttpRequest) -> HttpResponse {
    // Call the actual implementation here (to be implemented in another module)
    HttpResponse::Ok().body("challenge handler stub")
}
