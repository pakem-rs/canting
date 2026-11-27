use canting::cloner::laravel::model_parser::LaravelModelParser;
use canting::cloner::laravel::route_parser::LaravelRouteParser;
use canting::cloner::laravel::view_converter::LaravelViewConverter;
use canting::cloner::laravel::{CloneLaravelOptions, LaravelCloner};
use std::fs;

#[test]
fn test_laravel_model_parser_and_relation_extraction() {
    let php_code = r#"<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\SoftDeletes;

class Post extends Model
{
    use SoftDeletes;

    protected $table = 'blog_posts';
    protected $primaryKey = 'id';

    protected $fillable = [
        'title',
        'slug',
        'content',
        'user_id',
        'is_published',
        'views_count',
    ];

    protected $casts = [
        'is_published' => 'boolean',
        'views_count' => 'integer',
    ];

    public function user()
    {
        return $this->belongsTo(User::class, 'user_id');
    }

    public function comments()
    {
        return $this->hasMany(Comment::class, 'post_id');
    }

    public function tags()
    {
        return $this->belongsToMany(Tag::class, 'post_tag', 'post_id', 'tag_id');
    }

    public function featuredImage()
    {
        return $this->hasOne(Image::class, 'post_id');
    }
}
"#;

    let parsed = LaravelModelParser::parse(php_code).expect("Harus berhasil parsing Model Laravel");

    assert_eq!(parsed.class_name, "Post");
    assert_eq!(parsed.table_name, "blog_posts");
    assert_eq!(parsed.primary_key, "id");
    assert!(parsed.has_soft_deletes);
    assert!(parsed.has_timestamps);
    assert_eq!(parsed.fillable.len(), 6);
    assert_eq!(parsed.relations.len(), 4);

    let parser = LaravelModelParser;
    let rust_code = parser.generate_rust_code(&parsed);

    // Verifikasi atribut dan derive Pakem
    assert!(rust_code.contains("#[table(\"blog_posts\")]"));
    assert!(rust_code.contains("pub struct Post {"));
    assert!(rust_code.contains("pub is_published: bool,"));
    assert!(rust_code.contains("pub views_count: i64,"));
    assert!(rust_code.contains("pub deleted_at: Option<String>,"));

    // Verifikasi relasi macro
    assert!(rust_code.contains("belongs_to!(User, \"user_id\");"));
    assert!(rust_code.contains("has_many!(Comment, \"post_id\");"));
    assert!(rust_code.contains("belongs_to_many!(Tag, \"post_tag\", \"post_id\", \"tag_id\");"));
    assert!(rust_code.contains("has_one!(Image, \"post_id\");"));
}

#[test]
fn test_laravel_route_parser_and_resource_expansion() {
    let php_routes = r#"<?php

use Illuminate\Support\Facades\Route;
use App\Http\Controllers\HomeController;
use App\Http\Controllers\PostController;

Route::get('/', [HomeController::class, 'index']);
Route::post('/contact', [HomeController::class, 'submitContact']);

Route::resource('posts', PostController::class);
"#;

    let routes = LaravelRouteParser::parse(php_routes);
    // 2 explicit routes + 7 resource routes = 9 routes
    assert_eq!(routes.len(), 9);

    assert_eq!(routes[0].path, "/");
    assert_eq!(routes[0].method, "GET");
    assert_eq!(routes[0].controller, "HomeController");
    assert_eq!(routes[0].action, "index");

    assert_eq!(routes[1].path, "/contact");
    assert_eq!(routes[1].method, "POST");

    // Pastikan resource routes posts terurai
    let post_routes: Vec<_> = routes.iter().filter(|r| r.controller == "PostController").collect();
    assert_eq!(post_routes.len(), 7);

    let parser = LaravelRouteParser;
    let rust_routes = parser.generate_rust_routes(&routes);

    assert!(rust_routes.contains("pub fn register_routes(mut app: Router) -> Router {"));
    assert!(rust_routes.contains("app = app.route(\"/\", get(home_controller::index));"));
    assert!(rust_routes.contains("app = app.route(\"/contact\", post(home_controller::submit_contact));"));
    assert!(rust_routes.contains("app = app.route(\"/posts\", get(post_controller::index));"));
    assert!(rust_routes.contains("app = app.route(\"/posts/{id}\", get(post_controller::show));"));
}

#[test]
fn test_laravel_view_converter_syntax_translation() {
    let blade_php = r#"@extends('layouts.app')

@section('content')
<div class="container">
    <h1>{{ $post->title }}</h1>
    <p>Penulis: {{ $post->user->name }}</p>

    <ul>
        @foreach($comments as $comment)
            <li>
                Item ke-{{ $loop->iteration }}: {{ $comment['body'] }}
            </li>
        @endforeach
    </ul>

    <form action="/posts/{{ $post->id }}" method="POST">
        @csrf
        @method('DELETE')
        <button type="submit">Hapus</button>
    </form>
</div>
@endsection
"#;

    let converted = LaravelViewConverter::convert(blade_php);

    // Pastikan $post->title -> $post.title
    assert!(converted.contains("{{ $post.title }}"));
    assert!(converted.contains("{{ $post.user.name }}"));
    // Pastikan $loop->iteration -> $loop.iteration
    assert!(converted.contains("$loop.iteration"));
    // Pastikan $comment['body'] -> $comment.body
    assert!(converted.contains("{{ $comment.body }}"));
    // Pastikan @csrf dan @method dipertahankan
    assert!(converted.contains("@csrf"));
    assert!(converted.contains("@method('DELETE')"));
}

#[test]
fn test_full_laravel_project_cloner_orchestration() {
    let temp_laravel = "./test_laravel_dummy_app";
    let temp_pakem_output = "./test_pakem_dummy_output";

    let _ = fs::remove_dir_all(temp_laravel);
    let _ = fs::remove_dir_all(temp_pakem_output);

    // Siapkan skeleton dummy Laravel
    fs::create_dir_all(format!("{}/app/Models", temp_laravel)).unwrap();
    fs::create_dir_all(format!("{}/app/Http/Controllers", temp_laravel)).unwrap();
    fs::create_dir_all(format!("{}/routes", temp_laravel)).unwrap();
    fs::create_dir_all(format!("{}/resources/views/posts", temp_laravel)).unwrap();

    // 1. Model dummy
    fs::write(
        format!("{}/app/Models/Article.php", temp_laravel),
        r#"<?php
namespace App\Models;
use Illuminate\Database\Eloquent\Model;

class Article extends Model
{
    protected $table = 'articles';
    protected $fillable = ['title', 'content', 'user_id'];

    public function user() {
        return $this->belongsTo(User::class);
    }
}
"#,
    ).unwrap();

    // 2. Controller dummy
    fs::write(
        format!("{}/app/Http/Controllers/ArticleController.php", temp_laravel),
        r#"<?php
namespace App\Http\Controllers;

class ArticleController extends Controller
{
    public function index() {}
    public function show($id) {}
    public function store() {}
}
"#,
    ).unwrap();

    // 3. Route dummy
    fs::write(
        format!("{}/routes/web.php", temp_laravel),
        r#"<?php
use App\Http\Controllers\ArticleController;
Route::get('/articles', [ArticleController::class, 'index']);
Route::get('/articles/{id}', [ArticleController::class, 'show']);
"#,
    ).unwrap();

    // 4. View dummy
    fs::write(
        format!("{}/resources/views/posts/index.blade.php", temp_laravel),
        "<h1>{{ $article->title }}</h1>",
    ).unwrap();

    // Eksekusi cloner
    let options = CloneLaravelOptions {
        path: temp_laravel.to_string(),
        dry_run: false,
        models_only: false,
        routes_only: false,
        views_only: false,
        with_controllers: true,
        output: temp_pakem_output.to_string(),
    };

    let result = LaravelCloner::run(options);
    assert!(result.is_ok(), "Eksekusi LaravelCloner harus sukses");

    // Verifikasi berkas hasil kloning
    assert!(fs::metadata(format!("{}/src/app/models/article.rs", temp_pakem_output)).is_ok());
    assert!(fs::metadata(format!("{}/src/app/models/mod.rs", temp_pakem_output)).is_ok());
    assert!(fs::metadata(format!("{}/routes/web.rs", temp_pakem_output)).is_ok());
    assert!(fs::metadata(format!("{}/resources/views/posts/index.blade.rs", temp_pakem_output)).is_ok());
    assert!(fs::metadata(format!("{}/src/app/controllers/article_controller.rs", temp_pakem_output)).is_ok());

    // Bersihkan temporary directory
    let _ = fs::remove_dir_all(temp_laravel);
    let _ = fs::remove_dir_all(temp_pakem_output);
}
