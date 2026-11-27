use regex::Regex;

pub struct LaravelViewConverter;

impl LaravelViewConverter {
    /// Mengonversi sintaks template Blade PHP menjadi Blade Pakem (.blade.rs)
    pub fn convert(blade_php_content: &str) -> String {
        let mut content = blade_php_content.to_string();

        // 1. Ubah $loop->index, $loop->first, $loop->last, $loop->iteration menjadi $loop.property
        let loop_re = Regex::new(r#"\$loop->([A-Za-z0-9_]+)"#).unwrap();
        content = loop_re.replace_all(&content, "$$loop.$1").to_string();

        // 2. Ubah akses objek panah PHP berantai: $post->title atau $post->user->name menjadi $post.title / $post.user.name
        let arrow_chain_re = Regex::new(r#"(\$[A-Za-z0-9_]+(?:->[A-Za-z0-9_]+)+)"#).unwrap();
        content = arrow_chain_re
            .replace_all(&content, |caps: &regex::Captures| {
                let matched = caps.get(1).unwrap().as_str();
                matched.replace("->", ".")
            })
            .to_string();

        // 3. Ubah akses array bracket PHP: $item['key'] atau $item["key"] menjadi $item.key
        let bracket_re = Regex::new(r#"\$([A-Za-z0-9_]+)\s*\[\s*['"]([A-Za-z0-9_]+)['"]\s*\]"#).unwrap();
        content = bracket_re.replace_all(&content, "$$$1.$2").to_string();

        // 4. Ubah PHP 8 null-safe operator ?-> menjadi .
        let null_safe_re = Regex::new(r#"\?->"#).unwrap();
        content = null_safe_re.replace_all(&content, ".").to_string();

        // 5. Ubah @session('status') menjadi @if(session('status')) dan @endsession menjadi @endif
        let session_open_re = Regex::new(r#"@session\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();
        content = session_open_re.replace_all(&content, "@if(session('$1'))").to_string();
        content = content.replace("@endsession", "@endif");

        // 6. Netralkan blok @php ... @endphp agar kode PHP mentah tidak bocor ke output HTML
        let php_block_re = Regex::new(r#"(?s)@php(.*?)@endphp"#).unwrap();
        content = php_block_re.replace_all(&content, "<!-- [Pakem Clone]: PHP block neutralized -->").to_string();

        // 7. Pastikan @csrf tetap ada pada tag form yang memiliki method POST
        // (Pakem Blade Engine mendukung direktif @csrf asli)

        content
    }
}
