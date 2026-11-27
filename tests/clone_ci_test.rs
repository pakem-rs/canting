use canting::cloner::ci::model_parser::CiModelParser;
use canting::cloner::ci::route_parser::CiRouteParser;
use canting::cloner::ci::view_converter::CiViewConverter;
use canting::cloner::ci::{CiCloner, CloneCiOptions};
use std::fs;

#[test]
fn test_ci3_and_ci4_model_parser() {
    // 1. CI3 Model: User_model
    let ci3_code = r#"<?php
defined('BASEPATH') OR exit('No direct script access allowed');

class User_model extends CI_Model {
    protected $table = 'users';

    public function insert($data) {
        return $this->db->insert('users', [
            'name' => $data['name'],
            'email' => $data['email'],
            'role' => $data['role']
        ]);
    }
}
"#;

    let parsed_ci3 = CiModelParser::parse(ci3_code).expect("Harus berhasil parsing CI3 Model");
    assert_eq!(parsed_ci3.class_name, "User_model");
    assert_eq!(parsed_ci3.table_name, "users");
    assert_eq!(CiModelParser::clean_model_name(&parsed_ci3.class_name), "User");

    let parser = CiModelParser;
    let rust_code_ci3 = parser.generate_rust_code(&parsed_ci3);
    assert!(rust_code_ci3.contains("#[table(\"users\")]"));
    assert!(rust_code_ci3.contains("pub struct User {"));
    assert!(rust_code_ci3.contains("pub id: i64,"));
    assert!(rust_code_ci3.contains("pub name: String,"));
    assert!(rust_code_ci3.contains("pub email: String,"));

    // 2. CI4 Model: ProductModel
    let ci4_code = r#"<?php
namespace App\Models;
use CodeIgniter\Model;

class ProductModel extends Model {
    protected $table = 'products';
    protected $primaryKey = 'id';
    protected $allowedFields = ['name', 'price', 'stock', 'is_active'];
    protected $useTimestamps = true;
}
"#;

    let parsed_ci4 = CiModelParser::parse(ci4_code).expect("Harus berhasil parsing CI4 Model");
    assert_eq!(parsed_ci4.class_name, "ProductModel");
    assert_eq!(parsed_ci4.table_name, "products");
    assert_eq!(CiModelParser::clean_model_name(&parsed_ci4.class_name), "Product");

    let rust_code_ci4 = parser.generate_rust_code(&parsed_ci4);
    assert!(rust_code_ci4.contains("#[table(\"products\")]"));
    assert!(rust_code_ci4.contains("pub struct Product {"));
    assert!(rust_code_ci4.contains("pub price: f64,"));
    assert!(rust_code_ci4.contains("pub stock: i64,"));
    assert!(rust_code_ci4.contains("pub is_active: bool,"));
    assert!(rust_code_ci4.contains("pub created_at: Option<String>,"));
}

#[test]
fn test_ci_route_parser_ci3_and_ci4() {
    // 1. CI3 Routes syntax
    let ci3_routes = r#"<?php
defined('BASEPATH') OR exit('No direct script access allowed');

$route['default_controller'] = 'welcome';
$route['users'] = 'users/index';
$route['users/(:num)'] = 'users/show/$1';
$route['catalog/(:any)'] = 'catalog/view/$1';
"#;

    let routes_ci3 = CiRouteParser::parse(ci3_routes);
    assert_eq!(routes_ci3.len(), 4);

    assert_eq!(routes_ci3[0].path, "/");
    assert_eq!(routes_ci3[0].controller, "welcome");

    assert_eq!(routes_ci3[1].path, "/users");
    assert_eq!(routes_ci3[1].action, "index");

    assert_eq!(routes_ci3[2].path, "/users/{id}");
    assert_eq!(routes_ci3[2].action, "show");

    assert_eq!(routes_ci3[3].path, "/catalog/{slug}");

    // 2. CI4 Routes syntax
    let ci4_routes = r#"<?php
$routes->get('/', 'Home::index');
$routes->get('orders', 'Orders::index');
$routes->post('orders', 'Orders::create');
$routes->get('orders/(:segment)', 'Orders::show/$1');
"#;

    let routes_ci4 = CiRouteParser::parse(ci4_routes);
    assert_eq!(routes_ci4.len(), 4);
    assert_eq!(routes_ci4[1].path, "/orders");
    assert_eq!(routes_ci4[2].method, "POST");
    assert_eq!(routes_ci4[3].path, "/orders/{id}");
}

#[test]
fn test_ci_view_converter_to_blade_pakem() {
    let ci_view = r#"<!DOCTYPE html>
<html>
<head>
    <title><?= $page_title ?></title>
</head>
<body>
    <h1>Selamat Datang, <?= $user->name ?></h1>

    <?= form_open('auth/login'); ?>
        <input type="text" name="email" value="<?= $old_email ?>">
        <button type="submit">Login</button>
    <?= form_close(); ?>

    <?php if ($is_admin): ?>
        <p>Halo Administrator!</p>
    <?php elseif ($is_staff): ?>
        <p>Halo Staff!</p>
    <?php else: ?>
        <p>Halo Pengguna Biasa!</p>
    <?php endif; ?>

    <ul>
        <?php foreach ($products as $p): ?>
            <li><?= $p['name'] ?> - Rp <?= $p['price'] ?></li>
        <?php endforeach; ?>
    </ul>
</body>
</html>"#;

    let converted = CiViewConverter::convert(ci_view);

    // 1. Tag echo <?= ... ?> -> {{ ... }}
    assert!(converted.contains("{{ $page_title }}"));
    assert!(converted.contains("{{ $user.name }}"));
    assert!(converted.contains("{{ $p.name }}"));
    assert!(converted.contains("{{ $p.price }}"));

    // 2. Form helper form_open -> <form ...> @csrf
    assert!(converted.contains("<form action=\"/auth/login\" method=\"POST\">"));
    assert!(converted.contains("@csrf"));
    assert!(converted.contains("</form>"));

    // 3. Kontrol alur IF
    assert!(converted.contains("@if($is_admin)"));
    assert!(converted.contains("@elseif($is_staff)"));
    assert!(converted.contains("@else"));
    assert!(converted.contains("@endif"));

    // 4. Kontrol alur FOREACH
    assert!(converted.contains("@foreach($products as $p)"));
    assert!(converted.contains("@endforeach"));
}

#[test]
fn test_full_ci_project_cloner_orchestration() {
    let temp_ci = "./test_ci_dummy_app";
    let temp_pakem_output = "./test_pakem_ci_output";

    let _ = fs::remove_dir_all(temp_ci);
    let _ = fs::remove_dir_all(temp_pakem_output);

    // Siapkan skeleton dummy CodeIgniter 3
    fs::create_dir_all(format!("{}/application/models", temp_ci)).unwrap();
    fs::create_dir_all(format!("{}/application/controllers", temp_ci)).unwrap();
    fs::create_dir_all(format!("{}/application/config", temp_ci)).unwrap();
    fs::create_dir_all(format!("{}/application/views/users", temp_ci)).unwrap();

    // 1. Model dummy CI3
    fs::write(
        format!("{}/application/models/Customer_model.php", temp_ci),
        r#"<?php
class Customer_model extends CI_Model {
    protected $table = 'customers';
    public function get_all() {
        return $this->db->get('customers')->result();
    }
}
"#,
    ).unwrap();

    // 2. Controller dummy CI3
    fs::write(
        format!("{}/application/controllers/Customer.php", temp_ci),
        r#"<?php
class Customer extends CI_Controller {
    public function index() {}
    public function detail($id) {}
    public function save() {}
}
"#,
    ).unwrap();

    // 3. Routes dummy CI3
    fs::write(
        format!("{}/application/config/routes.php", temp_ci),
        r#"<?php
$route['customers'] = 'customer/index';
$route['customers/(:num)'] = 'customer/detail/$1';
"#,
    ).unwrap();

    // 4. View dummy CI3
    fs::write(
        format!("{}/application/views/users/list.php", temp_ci),
        "<h1>Daftar <?= $title ?></h1>",
    ).unwrap();

    // Jalankan Cloner
    let options = CloneCiOptions {
        path: temp_ci.to_string(),
        dry_run: false,
        models_only: false,
        routes_only: false,
        views_only: false,
        with_controllers: true,
        output: temp_pakem_output.to_string(),
    };

    let result = CiCloner::run(options);
    assert!(result.is_ok(), "Eksekusi CiCloner harus sukses");

    // Verifikasi berkas hasil kloning
    assert!(fs::metadata(format!("{}/src/app/models/customer.rs", temp_pakem_output)).is_ok());
    assert!(fs::metadata(format!("{}/src/app/models/mod.rs", temp_pakem_output)).is_ok());
    assert!(fs::metadata(format!("{}/routes/web.rs", temp_pakem_output)).is_ok());
    assert!(fs::metadata(format!("{}/resources/views/users/list.blade.rs", temp_pakem_output)).is_ok());
    assert!(fs::metadata(format!("{}/src/app/controllers/customer_controller.rs", temp_pakem_output)).is_ok());

    // Bersihkan temporary directory
    let _ = fs::remove_dir_all(temp_ci);
    let _ = fs::remove_dir_all(temp_pakem_output);
}
