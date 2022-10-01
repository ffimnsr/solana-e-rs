use actix_web::{
    body::BoxBody,
    dev::ServiceResponse,
    error, get,
    http::{header::ContentType, StatusCode},
    middleware::{self, ErrorHandlerResponse, ErrorHandlers},
    web, App, Error, HttpResponse, HttpServer, Result,
};
use dotenv::dotenv;
use serde::Serialize;
use solana_e::crawler::SolanaCrawler;
use std::{collections::HashMap, env};
use tera::Tera;

#[get("/")]
async fn index(tmpl: web::Data<Tera>) -> Result<HttpResponse, Error> {
    let s = tmpl
        .render("index.html", &tera::Context::new())
        .map_err(|_| error::ErrorInternalServerError("Failed to load template error"))?;
    Ok(HttpResponse::Ok().content_type(ContentType::html()).body(s))
}

#[get("/solana_version")]
async fn solana_version() -> Result<HttpResponse, Error> {
    let url = "https://solitary-white-violet.solana-mainnet.quiknode.pro/";
    let crawler = SolanaCrawler::new(url);
    let res = crawler
        .get_version()
        .await
        .expect("solana_version: fatal error on fetching solana version.");

    #[derive(Serialize)]
    struct SolanaVersion {
        version: String,
    }

    let obj = SolanaVersion { version: res };

    let json = serde_json::to_string(&obj).unwrap();
    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(json))
}

#[get("/load_metadata")]
async fn load_metadata(query: web::Query<HashMap<String, String>>) -> Result<HttpResponse, Error> {
    let metadata_b64 = query
        .get("data")
        .ok_or(error::ErrorInternalServerError(
            "Failed to load template error",
        ))
        .map(|x| x.as_str())?;

    let metadata = base64::decode(metadata_b64)
        .map_err(|_| error::ErrorInternalServerError("Failed to decode metadata"))?;

    let metadata: Option<String> = bincode::deserialize(&metadata[..])
        .map_err(|_| error::ErrorInternalServerError("Failed to validate metadata"))?;

    let metadata_uri = metadata.ok_or(error::ErrorInternalServerError(
        "Failed to unwrap metadata location with value none",
    ))?;

    let metadata_body = reqwest::get(metadata_uri)
        .await
        .map_err(|err| error::ErrorInternalServerError(err))?
        .text()
        .await
        .map_err(|err| error::ErrorInternalServerError(err))?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(metadata_body))
}

#[get("/wallet")]
async fn wallet(
    tmpl: web::Data<Tera>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse, Error> {
    let account = query
        .get("account")
        .ok_or(error::ErrorInternalServerError(
            "Failed to load template error",
        ))
        .map(|x| x.as_str())?;

    let url = "https://solitary-white-violet.solana-mainnet.quiknode.pro/";
    let crawler = SolanaCrawler::new(url);
    let tokens = crawler.get_nfts_for_owner(account).await.unwrap_or(vec![]);

    let mut ctx = tera::Context::new();
    ctx.insert("tokens_len", &tokens.len());
    ctx.insert("tokens", &tokens);
    let body = tmpl
        .render("wallet.html", &ctx)
        .map_err(|_| error::ErrorInternalServerError("Failed to load template error"))?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "solana_e=trace");
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    log::info!("Listening on 0.0.0.0:8081!");

    let template_path = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*");
    log::info!("Loading template from path {}", template_path);

    HttpServer::new(|| {
        let tera = Tera::new(template_path).unwrap();

        App::new()
            .app_data(web::Data::new(tera))
            .wrap(middleware::Logger::default())
            .service(index)
            .service(wallet)
            .service(load_metadata)
            .service(solana_version)
            .service(web::scope("").wrap(error_handlers()))
    })
    .bind(("0.0.0.0", 8081))?
    .run()
    .await
}

fn error_handlers() -> ErrorHandlers<BoxBody> {
    ErrorHandlers::new().handler(StatusCode::NOT_FOUND, not_found)
}

fn not_found<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<BoxBody>> {
    let response = get_error_response(&res, "Page Not Found");

    Ok(ErrorHandlerResponse::Response(ServiceResponse::new(
        res.into_parts().0,
        response.map_into_left_body(),
    )))
}

fn get_error_response<B>(res: &ServiceResponse<B>, error: &str) -> HttpResponse {
    let request = res.request();

    let fallback = |e: &str| {
        HttpResponse::build(res.status())
            .content_type(ContentType::plaintext())
            .body(e.to_string())
    };

    let tera = request.app_data::<web::Data<Tera>>().map(|t| t.get_ref());
    match tera {
        Some(tera) => {
            let mut ctx = tera::Context::new();
            ctx.insert("error", error);
            ctx.insert("status_code", res.status().as_str());

            let body = tera.render("error.html", &ctx);
            match body {
                Ok(body) => HttpResponse::build(res.status())
                    .content_type(ContentType::html())
                    .body(body),
                Err(_) => fallback(error),
            }
        }
        None => fallback(error),
    }
}
