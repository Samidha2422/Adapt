#![allow(clippy::unwrap_used)]
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hre_core::EngineConfig;
use tower::ServiceExt;

#[tokio::test]
async fn dashboard_requires_moderator_and_cookie_mutations_require_origin() {
    let config = EngineConfig {
        salt: "a".repeat(32),
        api_keys: vec!["client".into()],
        moderator_tokens: vec!["moderator".into()],
        models_dir: "target/absent-dashboard-test-models".into(),
        ..EngineConfig::default()
    };
    let app = hre_api::router(config, true).await.unwrap();
    for key in [None, Some("client")] {
        let mut req = Request::get("/dashboard");
        if let Some(key) = key {
            req = req.header("x-api-key", key);
        }
        let response = app
            .clone()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers()["location"], "/dashboard/login");
    }
    let response = app
        .clone()
        .oneshot(
            Request::post("/dashboard/login")
                .header("host", "localhost:8080")
                .header("origin", "http://localhost:8080")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("token=moderator"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let response = app
        .clone()
        .oneshot(
            Request::get("/dashboard")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let response = app
        .clone()
        .oneshot(
            Request::post("/dashboard/logout")
                .header("cookie", &cookie)
                .header("host", "localhost:8080")
                .header("origin", "http://evil.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = app
        .clone()
        .oneshot(
            Request::post("/dashboard/logout")
                .header("cookie", &cookie)
                .header("host", "localhost:8080")
                .header("origin", "http://localhost:8080")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let response = app
        .oneshot(
            Request::get("/dashboard")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}
