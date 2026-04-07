use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateItem {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateItem {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct PatchItem {
    pub name: Option<String>,
    pub description: Option<String>,
}

pub type Db = Arc<Mutex<HashMap<String, Item>>>;

pub fn create_app(db: Db) -> Router {
    Router::new()
        .route("/items", get(list_items).post(create_item))
        .route(
            "/items/{id}",
            get(get_item).put(update_item).patch(patch_item).delete(delete_item),
        )
        .with_state(db)
}

#[tokio::main]
async fn main() {
    let db: Db = Arc::new(Mutex::new(HashMap::new()));
    let app = create_app(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn list_items(State(db): State<Db>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    let items: Vec<Item> = db.values().cloned().collect();
    Json(items)
}

async fn create_item(
    State(db): State<Db>,
    Json(payload): Json<CreateItem>,
) -> impl IntoResponse {
    let item = Item {
        id: Uuid::new_v4().to_string(),
        name: payload.name,
        description: payload.description,
    };
    let mut db = db.lock().unwrap();
    db.insert(item.id.clone(), item.clone());
    (StatusCode::CREATED, Json(item))
}

async fn get_item(State(db): State<Db>, Path(id): Path<String>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.get(&id) {
        Some(item) => (StatusCode::OK, Json(item.clone())).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn update_item(
    State(db): State<Db>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateItem>,
) -> impl IntoResponse {
    let mut db = db.lock().unwrap();
    match db.get_mut(&id) {
        Some(item) => {
            item.name = payload.name;
            item.description = payload.description;
            (StatusCode::OK, Json(item.clone())).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn patch_item(
    State(db): State<Db>,
    Path(id): Path<String>,
    Json(payload): Json<PatchItem>,
) -> impl IntoResponse {
    let mut db = db.lock().unwrap();
    match db.get_mut(&id) {
        Some(item) => {
            if let Some(name) = payload.name {
                item.name = name;
            }
            if let Some(description) = payload.description {
                item.description = description;
            }
            (StatusCode::OK, Json(item.clone())).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn delete_item(State(db): State<Db>, Path(id): Path<String>) -> impl IntoResponse {
    let mut db = db.lock().unwrap();
    match db.remove(&id) {
        Some(_) => StatusCode::NO_CONTENT.into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    fn test_db() -> Db {
        Arc::new(Mutex::new(HashMap::new()))
    }

    async fn body_to_json(body: Body) -> Value {
        let bytes = body.collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn body_to_string(body: Body) -> String {
        let bytes = body.collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn test_list_items_empty() {
        let app = create_app(test_db());
        let response = app
            .oneshot(Request::builder().uri("/items").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_to_json(response.into_body()).await;
        assert_eq!(json, json!([]));
    }

    #[tokio::test]
    async fn test_create_item() {
        let app = create_app(test_db());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/items")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"name": "sword", "description": "sharp"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let json = body_to_json(response.into_body()).await;
        assert_eq!(json["name"], "sword");
        assert_eq!(json["description"], "sharp");
        assert!(json["id"].is_string());
    }

    #[tokio::test]
    async fn test_get_item_not_found() {
        let app = create_app(test_db());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/items/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_item() {
        let db = test_db();
        let id = Uuid::new_v4().to_string();
        db.lock().unwrap().insert(
            id.clone(),
            Item { id: id.clone(), name: "shield".into(), description: "round".into() },
        );

        let app = create_app(db);
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/items/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_to_json(response.into_body()).await;
        assert_eq!(json["name"], "shield");
    }

    #[tokio::test]
    async fn test_update_item() {
        let db = test_db();
        let id = Uuid::new_v4().to_string();
        db.lock().unwrap().insert(
            id.clone(),
            Item { id: id.clone(), name: "axe".into(), description: "heavy".into() },
        );

        let app = create_app(db);
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/items/{id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"name": "great axe", "description": "very heavy"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_to_json(response.into_body()).await;
        assert_eq!(json["name"], "great axe");
        assert_eq!(json["description"], "very heavy");
    }

    #[tokio::test]
    async fn test_update_item_not_found() {
        let app = create_app(test_db());
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/items/ghost")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"name": "x", "description": "y"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_patch_item_partial() {
        let db = test_db();
        let id = Uuid::new_v4().to_string();
        db.lock().unwrap().insert(
            id.clone(),
            Item { id: id.clone(), name: "bow".into(), description: "wooden".into() },
        );

        let app = create_app(db);
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/items/{id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"name": "longbow"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_to_json(response.into_body()).await;
        assert_eq!(json["name"], "longbow");
        assert_eq!(json["description"], "wooden");
    }

    #[tokio::test]
    async fn test_patch_item_not_found() {
        let app = create_app(test_db());
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/items/ghost")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"name": "x"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_item() {
        let db = test_db();
        let id = Uuid::new_v4().to_string();
        db.lock().unwrap().insert(
            id.clone(),
            Item { id: id.clone(), name: "dagger".into(), description: "small".into() },
        );

        let app = create_app(db);
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/items/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let body = body_to_string(response.into_body()).await;
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn test_delete_item_not_found() {
        let app = create_app(test_db());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/items/ghost")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_items_after_create() {
        let db = test_db();
        db.lock().unwrap().insert(
            "1".into(),
            Item { id: "1".into(), name: "item1".into(), description: "desc1".into() },
        );
        db.lock().unwrap().insert(
            "2".into(),
            Item { id: "2".into(), name: "item2".into(), description: "desc2".into() },
        );

        let app = create_app(db);
        let response = app
            .oneshot(Request::builder().uri("/items").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_to_json(response.into_body()).await;
        assert_eq!(json.as_array().unwrap().len(), 2);
    }
}
