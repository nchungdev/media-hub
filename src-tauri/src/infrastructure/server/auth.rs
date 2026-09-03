use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{Html, IntoResponse, Response},
};
use std::env;

pub async fn auth_middleware(req: Request, next: Next) -> Response {
    let auth_token = env::var("MEDIA_HUB_TOKEN").unwrap_or_default().trim().to_string();
    if auth_token.is_empty() {
        return next.run(req).await;
    }

    // 1. Check header
    if let Some(h) = req.headers().get("X-Media-Hub-Token") {
        if let Ok(val) = h.to_str() {
            if val.trim() == auth_token {
                return next.run(req).await;
            }
        }
    }

    // 2. Check Cookie
    if let Some(cookie_header) = req.headers().get("Cookie") {
        if let Ok(cookies) = cookie_header.to_str() {
            for part in cookies.split(';') {
                let mut kv = part.trim().splitn(2, '=');
                if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                    if k.trim() == "mh_token" && v.trim() == auth_token {
                        return next.run(req).await;
                    }
                }
            }
        }
    }

    // 3. Check Query parameter ?k=...
    let mut set_cookie = false;
    if let Some(query) = req.uri().query() {
        for pair in query.split('&') {
            let mut kv = pair.splitn(2, '=');
            if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                if k == "k" && v == auth_token {
                    set_cookie = true;
                    break;
                }
            }
        }
    }

    if set_cookie {
        let mut response = next.run(req).await;
        if let Ok(cookie_val) = format!("mh_token={}; Path=/; HttpOnly; SameSite=Lax", auth_token).parse() {
            response.headers_mut().insert(axum::http::header::SET_COOKIE, cookie_val);
        }
        return response;
    }

    (
        StatusCode::UNAUTHORIZED,
        Html(
            "<h1>401 - Media Hub</h1><p>Dashboard dang chay o che do cong khai. \
            Hay mo link kem token: <code>?k=&lt;token&gt;</code></p>",
        ),
    )
        .into_response()
}
