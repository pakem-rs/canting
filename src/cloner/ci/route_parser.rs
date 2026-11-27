use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCiRoute {
    pub method: String, // "GET", "POST", "PUT", "DELETE"
    pub path: String,   // "/users", "/users/{id}"
    pub controller: String, // "Users"
    pub action: String, // "index", "show"
}

pub struct CiRouteParser;

impl CiRouteParser {
    /// Mem-parsing file routes CodeIgniter (baik format CI3 $route[...] maupun CI4 $routes->get(...))
    pub fn parse(php_content: &str) -> Vec<ParsedCiRoute> {
        let mut routes = Vec::new();

        // 1. Format CI4: $routes->get('users', 'Users::index');
        let ci4_re = Regex::new(
            r#"\$routes->(get|post|put|delete|patch)\(\s*['"]([^'"]+)['"]\s*,\s*['"]([A-Za-z0-9_]+)::([A-Za-z0-9_]+)(?:/[^'"]*)?['"]"#
        ).unwrap();

        for cap in ci4_re.captures_iter(php_content) {
            let method = cap.get(1).map(|m| m.as_str().to_uppercase()).unwrap_or_default();
            let raw_path = cap.get(2).map(|m| m.as_str()).unwrap_or_default();
            let controller = cap.get(3).map(|m| m.as_str().to_string()).unwrap_or_default();
            let action = cap.get(4).map(|m| m.as_str().to_string()).unwrap_or_default();

            let path = Self::normalize_ci_path(raw_path);
            routes.push(ParsedCiRoute {
                method,
                path,
                controller,
                action,
            });
        }

        // 2. Format CI3: $route['users/(:num)'] = 'users/show/$1';
        let ci3_re = Regex::new(
            r#"\$route\s*\[\s*['"]([^'"]+)['"]\s*\]\s*=\s*['"]([^'"]+)['"]\s*;"#
        ).unwrap();

        for cap in ci3_re.captures_iter(php_content) {
            let key = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
            let val = cap.get(2).map(|m| m.as_str()).unwrap_or_default();

            // Tangani default_controller (misal 'welcome' -> GET '/' => welcome::index)
            if key == "default_controller" {
                let parts: Vec<&str> = val.split('/').collect();
                let ctrl = parts.first().unwrap_or(&"welcome").to_string();
                let act = parts.get(1).unwrap_or(&"index").to_string();
                routes.push(ParsedCiRoute {
                    method: "GET".to_string(),
                    path: "/".to_string(),
                    controller: ctrl,
                    action: act,
                });
                continue;
            }

            if key == "404_override" || key == "translate_uri_dashes" {
                continue;
            }

            // Normalisasi target controller/action (misal 'users/show/$1' -> controller: 'users', action: 'show')
            let target_parts: Vec<&str> = val.split('/').collect();
            if target_parts.is_empty() {
                continue;
            }

            let controller = target_parts[0].to_string();
            let action = if target_parts.len() > 1 && !target_parts[1].starts_with('$') {
                target_parts[1].to_string()
            } else {
                "index".to_string()
            };

            let path = Self::normalize_ci_path(key);
            routes.push(ParsedCiRoute {
                method: "GET".to_string(), // Default di CI3
                path,
                controller,
                action,
            });
        }

        routes
    }

    /// Mengonversi placeholder wildcard CI (:num, :any, :segment) ke standar RESTful {id} / {slug}
    fn normalize_ci_path(raw: &str) -> String {
        let clean = raw.trim().trim_matches('/');
        if clean.is_empty() {
            return "/".to_string();
        }

        let mut path = format!("/{}", clean);
        path = path.replace("(:num)", "{id}");
        path = path.replace("(:segment)", "{id}");
        path = path.replace("(:any)", "{slug}");
        path
    }

    /// Menghasilkan file routes Rust Pakem (routes/web.rs)
    pub fn generate_rust_routes(&self, routes: &[ParsedCiRoute]) -> String {
        let mut code = String::new();
        code.push_str("use pakem::prelude::*;\n");
        code.push_str("use crate::app::controllers::*;\n\n");
        code.push_str("/// Definisi rute hasil kloning dari CodeIgniter Routes\n");
        code.push_str("pub fn register_routes(mut app: Router) -> Router {\n");

        if routes.is_empty() {
            code.push_str("    // Belum ada rute terdeteksi\n");
            code.push_str("    app\n");
            code.push_str("}\n");
            return code;
        }

        for r in routes {
            let ctrl_mod = Self::to_controller_module_name(&r.controller);
            let action = Self::to_snake_case(&r.action);
            let method_fn = r.method.to_lowercase();

            code.push_str(&format!(
                "    app = app.route(\"{}\", {}({}::{}));\n",
                r.path, method_fn, ctrl_mod, action
            ));
        }

        code.push_str("\n    app\n");
        code.push_str("}\n");
        code
    }

    pub fn to_controller_module_name(controller_class: &str) -> String {
        let mut snake = Self::to_snake_case(controller_class);
        if !snake.ends_with("_controller") {
            snake = format!("{}_controller", snake);
        }
        snake
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
