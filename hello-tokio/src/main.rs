use axum::{routing::get, Router, extract::Path};
use askama::Template;
use askama_web::WebTemplate;

#[derive(Template, WebTemplate)]
#[template(source = "<h1>Hello {{ name }}!</h1>", ext="html")]
struct HelloTemplate {
    name: String,
}
//struct HelloTemplate<'a> {
//    name: &'a str,
//}

async fn hello(name: Option<Path<String>>) -> HelloTemplate {
    HelloTemplate {
        name: name.map(|Path(n)| n).unwrap_or_else(|| "stranger".to_string()),
    }
}

#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new()
        .route("/{name}", get(hello))
        .route("/", get(hello));

    // run it
    //let listener = tokio::net::TcpListener::bind("0.0.0.0:20174")
    let listener = tokio::net::TcpListener::bind("[::]:20174")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    let _ = axum::serve(listener, app).await;
}


