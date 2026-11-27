use canting::cloner::api::endpoint_parser::ApiEndpointParser;
use canting::cloner::api::schema_parser::ApiSchemaParser;
use canting::cloner::api::{ApiCloner, CloneApiOptions};
use serde_json::json;
use std::fs;

#[test]
fn test_openapi_schema_to_rust_dto() {
    let spec = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Sample E-Commerce API",
            "version": "1.0.0"
        },
        "components": {
            "schemas": {
                "ProductDto": {
                    "type": "object",
                    "required": ["id", "name", "price"],
                    "properties": {
                        "id": { "type": "integer", "format": "int64" },
                        "name": { "type": "string" },
                        "price": { "type": "number", "format": "double" },
                        "is_active": { "type": "boolean" },
                        "tags": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                }
            }
        }
    });

    let schemas = ApiSchemaParser::parse_schemas(&spec);
    assert_eq!(schemas.len(), 1);

    let product = schemas.get("ProductDto").expect("ProductDto harus ditemukan");
    assert_eq!(product.name, "Product");
    assert_eq!(product.fields.len(), 5);

    let parser = ApiSchemaParser;
    let rust_code = parser.generate_rust_dto(product);

    assert!(rust_code.contains("pub struct Product {"));
    assert!(rust_code.contains("pub id: i64,"));
    assert!(rust_code.contains("pub name: String,"));
    assert!(rust_code.contains("pub price: f64,"));
    assert!(rust_code.contains("pub is_active: Option<bool>,"));
    assert!(rust_code.contains("pub tags: Option<Vec<String>>,"));
}

#[test]
fn test_openapi_paths_to_rust_routes_and_controllers() {
    let spec = json!({
        "openapi": "3.0.0",
        "paths": {
            "/api/v1/orders": {
                "get": {
                    "operationId": "listOrders",
                    "tags": ["Orders"],
                    "summary": "Daftar pesanan"
                },
                "post": {
                    "operationId": "createOrder",
                    "tags": ["Orders"],
                    "summary": "Buat pesanan baru"
                }
            },
            "/api/v1/orders/{id}": {
                "get": {
                    "operationId": "getOrderById",
                    "tags": ["Orders"]
                },
                "delete": {
                    "operationId": "cancelOrder",
                    "tags": ["Orders"]
                }
            }
        }
    });

    let endpoints = ApiEndpointParser::parse_paths(&spec);
    assert_eq!(endpoints.len(), 4);

    let routes_code = ApiEndpointParser::generate_rust_routes(&endpoints);
    assert!(routes_code.contains("pub fn register_api_routes(mut app: Router) -> Router {"));
    assert!(routes_code.contains("app = app.route(\"/api/v1/orders\", get(orders_controller::list_orders));"));
    assert!(routes_code.contains("app = app.route(\"/api/v1/orders\", post(orders_controller::create_order));"));
    assert!(routes_code.contains("app = app.route(\"/api/v1/orders/{id}\", get(orders_controller::get_order_by_id));"));
    assert!(routes_code.contains("app = app.route(\"/api/v1/orders/{id}\", delete(orders_controller::cancel_order));"));

    let controllers = ApiEndpointParser::generate_controllers_by_tag(&endpoints);
    let orders_ctrl = controllers.get("Orders").expect("Orders controller harus ada");

    assert!(orders_ctrl.contains("pub async fn list_orders(req: Request) -> PakemResult {"));
    assert!(orders_ctrl.contains("pub async fn create_order(req: Request) -> PakemResult {"));
    assert!(orders_ctrl.contains("pub async fn get_order_by_id(req: Request) -> PakemResult {"));
    assert!(orders_ctrl.contains("let _id = req.id_or_fail()?;"));
}

#[test]
fn test_full_api_cloner_orchestration() {
    let temp_spec_file = "./test_temp_swagger.json";
    let temp_pakem_output = "./test_pakem_api_output";

    let _ = fs::remove_file(temp_spec_file);
    let _ = fs::remove_dir_all(temp_pakem_output);

    let swagger_json = r#"{
      "swagger": "2.0",
      "info": {
        "title": "Golang Microservice API",
        "version": "2.1.0"
      },
      "definitions": {
        "Customer": {
          "type": "object",
          "required": ["id", "email"],
          "properties": {
            "id": { "type": "integer" },
            "email": { "type": "string" },
            "phone": { "type": "string" }
          }
        }
      },
      "paths": {
        "/api/customers": {
          "get": {
            "tags": ["Customers"],
            "operationId": "getCustomers"
          },
          "post": {
            "tags": ["Customers"],
            "operationId": "saveCustomer"
          }
        }
      }
    }"#;

    fs::write(temp_spec_file, swagger_json).unwrap();

    let options = CloneApiOptions {
        source: temp_spec_file.to_string(),
        dry_run: false,
        dto_only: false,
        routes_only: false,
        with_controllers: true,
        output: temp_pakem_output.to_string(),
    };

    let result = ApiCloner::run(options);
    assert!(result.is_ok(), "Eksekusi ApiCloner harus sukses");

    // Verifikasi berkas hasil kloning
    assert!(fs::metadata(format!("{}/src/app/dto/customer.rs", temp_pakem_output)).is_ok());
    assert!(fs::metadata(format!("{}/src/app/dto/mod.rs", temp_pakem_output)).is_ok());
    assert!(fs::metadata(format!("{}/routes/api.rs", temp_pakem_output)).is_ok());
    assert!(fs::metadata(format!("{}/src/app/controllers/customers_controller.rs", temp_pakem_output)).is_ok());

    // Bersihkan file sementara
    let _ = fs::remove_file(temp_spec_file);
    let _ = fs::remove_dir_all(temp_pakem_output);
}
