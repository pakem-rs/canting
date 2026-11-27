use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedLaravelRoute {
    pub method: String, // "GET", "POST", "PUT", "DELETE", "PATCH"
    pub path: String,   // "/posts", "/users/{id}"
    pub controller: String, // "PostController"
    pub action: String, // "index", "store", "show", dll.
}

pub struct LaravelRouteParser;

impl LaravelRouteParser {
    /// Mem-parsing konten file routes/web.php atau routes/api.php
    pub fn parse(php_content: &str) -> Vec<ParsedLaravelRoute> {
        let mut routes = Vec::new();

        // 1. Ekstrak Route::get, post, put, patch, delete dengan array syntax [Controller::class, 'method']
        // Contoh: Route::get('/posts/{id}', [PostController::class, 'show']);
        let action_re = Regex::new(
            r#"Route::(get|post|put|patch|delete)\(\s*['"]([^'"]+)['"]\s*,\s*\[\s*([A-Za-z0-9_\\]+)::class\s*,\s*['"]([A-Za-z0-9_]+)['"]\s*\]"#
        ).unwrap();

        for cap in action_re.captures_iter(php_content) {
            let method = cap.get(1).map(|m| m.as_str().to_uppercase()).unwrap_or_default();
            let path = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            let raw_ctrl = cap.get(3).map(|m| m.as_str()).unwrap_or_default();
            let controller = Self::clean_class_name(raw_ctrl);
            let action = cap.get(4).map(|m| m.as_str().to_string()).unwrap_or_default();

            routes.push(ParsedLaravelRoute {
                method,
                path,
                controller,
                action,
            });
        }

        // 2. Ekstrak string syntax 'PostController@index' (Laravel 5 - 7 legacy)
        // Contoh: Route::get('/posts', 'PostController@index');
        let string_syntax_re = Regex::new(
            r#"Route::(get|post|put|patch|delete)\(\s*['"]([^'"]+)['"]\s*,\s*['"]([A-Za-z0-9_\\]+)@([A-Za-z0-9_]+)['"]"#
        ).unwrap();

        for cap in string_syntax_re.captures_iter(php_content) {
            let method = cap.get(1).map(|m| m.as_str().to_uppercase()).unwrap_or_default();
            let path = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            let raw_ctrl = cap.get(3).map(|m| m.as_str()).unwrap_or_default();
            let controller = Self::clean_class_name(raw_ctrl);
            let action = cap.get(4).map(|m| m.as_str().to_string()).unwrap_or_default();

            routes.push(ParsedLaravelRoute {
                method,
                path,
                controller,
                action,
            });
        }

        // 3. Ekstrak Route::resource('posts', PostController::class)
        let resource_re = Regex::new(
            r#"Route::resource\(\s*['"]([^'"]+)['"]\s*,\s*([A-Za-z0-9_\\]+)::class"#
        ).unwrap();

        for cap in resource_re.captures_iter(php_content) {
            let resource = cap.get(1).map(|m| m.as_str().trim_matches('/')).unwrap_or_default();
            let raw_ctrl = cap.get(2).map(|m| m.as_str()).unwrap_or_default();
            let controller = Self::clean_class_name(raw_ctrl);

            // Expand 7 standard RESTful resource routes
            routes.push(ParsedLaravelRoute {
                method: "GET".to_string(),
                path: format!("/{}", resource),
                controller: controller.clone(),
                action: "index".to_string(),
            });
            routes.push(ParsedLaravelRoute {
                method: "GET".to_string(),
                path: format!("/{}/create", resource),
                controller: controller.clone(),
                action: "create".to_string(),
            });
            routes.push(ParsedLaravelRoute {
                method: "POST".to_string(),
                path: format!("/{}", resource),
                controller: controller.clone(),
                action: "store".to_string(),
            });
            routes.push(ParsedLaravelRoute {
                method: "GET".to_string(),
                path: format!("/{}/{{id}}", resource),
                controller: controller.clone(),
                action: "show".to_string(),
            });
            routes.push(ParsedLaravelRoute {
                method: "GET".to_string(),
                path: format!("/{}/{{id}}/edit", resource),
                controller: controller.clone(),
                action: "edit".to_string(),
            });
            routes.push(ParsedLaravelRoute {
                method: "PUT".to_string(),
                path: format!("/{}/{{id}}", resource),
                controller: controller.clone(),
                action: "update".to_string(),
            });
            routes.push(ParsedLaravelRoute {
                method: "DELETE".to_string(),
                path: format!("/{}/{{id}}", resource),
                controller: controller.clone(),
                action: "destroy".to_string(),
            });
        }

        routes
    }

    /// Menghasilkan file routes Rust Pakem (contoh: routes/web.rs)
    pub fn generate_rust_routes(&self, routes: &[ParsedLaravelRoute]) -> String {
        let mut code = String::new();
        code.push_str("use pakem::prelude::*;\n");
        code.push_str("use crate::app::controllers::*;\n\n");
        code.push_str("/// Definisi rute hasil kloning dari Laravel Routes\n");
        code.push_str("pub fn register_routes(mut app: Router) -> Router {\n");

        if routes.is_empty() {
            code.push_str("    // Belum ada rute yang terdeteksi\n");
            code.push_str("    app\n");
            code.push_str("}\n");
            return code;
        }

        for r in routes {
            let ctrl_mod = Self::to_controller_module_name(&r.controller);
            let action = Self::to_snake_case(&r.action);
            let method_fn = r.method.to_lowercase(); // get, post, put, delete

            code.push_str(&format!(
                "    app = app.route(\"{}\", {}({}::{}));\n",
                r.path, method_fn, ctrl_mod, action
            ));
        }

        code.push_str("\n    app\n");
        code.push_str("}\n");
        code
    }

    fn clean_class_name(raw: &str) -> String {
        raw.split('\\').last().unwrap_or(raw).trim().to_string()
    }

    pub fn to_controller_module_name(controller_class: &str) -> String {
        let snake = Self::to_snake_case(controller_class);
        if snake.ends_with("_controller") {
            snake
        } else {
            format!("{}_controller", snake)
        }
    }

    pub fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() {
                if i > 0 {
                    result.push('_');
                }
                result.push(ch.to_ascii_lowercase());
            } else {
                result.push(ch);
            }
        }
        result
    }
}
