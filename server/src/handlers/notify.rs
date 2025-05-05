use actix_web::{HttpRequest, HttpResponse};

pub async fn handle_notify(_req: HttpRequest) -> HttpResponse {
    // Call the actual implementation here (to be implemented in another module)
    HttpResponse::Ok().body("notify handler stub")
}
