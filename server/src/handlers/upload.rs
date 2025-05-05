use actix_web::{HttpRequest, HttpResponse};

pub async fn handle_upload(_req: HttpRequest) -> HttpResponse {
    // Call the actual implementation here (to be implemented in another module)
    HttpResponse::Created().body("upload handler stub")
}
