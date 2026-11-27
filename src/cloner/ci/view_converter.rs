use regex::Regex;

pub struct CiViewConverter;

impl CiViewConverter {
    /// Mengonversi template PHP native CodeIgniter menjadi Native Blade Pakem (.blade.rs)
    pub fn convert(php_view_content: &str) -> String {
        let mut content = php_view_content.to_string();

        // 1. Ubah form_open(...) CI helper menjadi tag form HTML dengan @csrf
        // Contoh: <?= form_open('login') ?> atau <?php echo form_open('login'); ?>
        let form_open_re = Regex::new(
            r#"<\?(?:=|php\s+echo)\s+form_open\(\s*['"]([^'"]*)['"]\s*\);?\s*\?>"#
        ).unwrap();
        content = form_open_re.replace_all(&content, "<form action=\"/$1\" method=\"POST\">\n    @csrf").to_string();

        // 2. Ubah form_close() menjadi </form>
        let form_close_re = Regex::new(
            r#"<\?(?:=|php\s+echo)\s+form_close\(\s*\);?\s*\?>"#
        ).unwrap();
        content = form_close_re.replace_all(&content, "</form>").to_string();

        // 3. Ubah echo ekspresi: <?= $var ?> atau <?php echo $var; ?> menjadi {{ $var }}
        let echo_re = Regex::new(
            r#"<\?(?:=|php\s+echo)\s+([^;?]+?);?\s*\?>"#
        ).unwrap();
        content = echo_re.replace_all(&content, "{{ $1 }}").to_string();

        // 4. Ubah kontrol alur IF / ELSEIF / ELSE / ENDIF
        let if_re = Regex::new(r#"<\?php\s+if\s*\((.*?)\)\s*:\s*\?>"#).unwrap();
        content = if_re.replace_all(&content, "@if($1)").to_string();

        let elseif_re = Regex::new(r#"<\?php\s+elseif\s*\((.*?)\)\s*:\s*\?>"#).unwrap();
        content = elseif_re.replace_all(&content, "@elseif($1)").to_string();

        let else_re = Regex::new(r#"<\?php\s+else\s*:\s*\?>"#).unwrap();
        content = else_re.replace_all(&content, "@else").to_string();

        let endif_re = Regex::new(r#"<\?php\s+endif;?\s*\?>"#).unwrap();
        content = endif_re.replace_all(&content, "@endif").to_string();

        // 5. Ubah kontrol alur FOREACH / ENDFOREACH
        let foreach_re = Regex::new(r#"<\?php\s+foreach\s*\((.*?)\)\s*:\s*\?>"#).unwrap();
        content = foreach_re.replace_all(&content, "@foreach($1)").to_string();

        let endforeach_re = Regex::new(r#"<\?php\s+endforeach;?\s*\?>"#).unwrap();
        content = endforeach_re.replace_all(&content, "@endforeach").to_string();

        // 6. Ubah akses objek panah PHP: $post->title -> $post.title
        let arrow_chain_re = Regex::new(r#"(\$[A-Za-z0-9_]+(?:->[A-Za-z0-9_]+)+)"#).unwrap();
        content = arrow_chain_re
            .replace_all(&content, |caps: &regex::Captures| {
                let matched = caps.get(1).unwrap().as_str();
                matched.replace("->", ".")
            })
            .to_string();

        // 7. Ubah akses array bracket PHP: $item['title'] -> $item.title
        let bracket_re = Regex::new(r#"\$([A-Za-z0-9_]+)\s*\[\s*['"]([A-Za-z0-9_]+)['"]\s*\]"#).unwrap();
        content = bracket_re.replace_all(&content, "$$$1.$2").to_string();

        content
    }
}
