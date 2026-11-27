use colored::*;
use rand::RngCore;
use std::fs;
use std::path::Path;

const B64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Mengkodekan byte menjadi string Base64 standar RFC 4648
pub fn encode_base64(bytes: &[u8]) -> String {
    let mut result = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 { chunk[1] as usize } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as usize } else { 0 };

        result.push(B64_CHARS[b0 >> 2] as char);
        result.push(B64_CHARS[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        if chunk.len() > 1 {
            result.push(B64_CHARS[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(B64_CHARS[b2 & 0x3F] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Menghasilkan 32-byte acak berkekuatan kriptografis (CSPRNG 256-bit)
pub fn generate_random_32_bytes() -> [u8; 32] {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

/// Menghasilkan string APP_KEY baru sesuai format (base64 ala Laravel atau hex)
pub fn generate_key_string(cipher: &str) -> String {
    let bytes = generate_random_32_bytes();
    match cipher.to_lowercase().as_str() {
        "hex" => {
            let mut s = String::with_capacity(64);
            for b in bytes {
                s.push_str(&format!("{:02x}", b));
            }
            s
        }
        _ => format!("base64:{}", encode_base64(&bytes)),
    }
}

/// Menulis atau memperbarui APP_KEY (dan opsi JWT_SECRET) di dalam berkas konfigurasi .env
pub fn set_key_in_env_file(
    env_path: &Path,
    key: &str,
    jwt_key: Option<&str>,
    force: bool,
) -> Result<(), String> {
    // 1. Jika berkas .env belum ada, coba salin dari .env.example
    if !env_path.exists() {
        let example_path = env_path.parent().unwrap_or_else(|| Path::new("")).join(".env.example");
        if example_path.exists() {
            fs::copy(&example_path, env_path)
                .map_err(|e| format!("Gagal menyalin .env.example ke {}: {}", env_path.display(), e))?;
        } else {
            // Buat berkas baru dengan template dasar
            let default_env = "APP_NAME=Pakem\nAPP_ENV=local\nAPP_KEY=\nAPP_DEBUG=true\nAPP_URL=http://localhost:8000\n";
            fs::write(env_path, default_env)
                .map_err(|e| format!("Gagal membuat berkas {}: {}", env_path.display(), e))?;
        }
    }

    // 2. Baca isi berkas .env
    let content = fs::read_to_string(env_path)
        .map_err(|e| format!("Gagal membaca berkas {}: {}", env_path.display(), e))?;

    // 3. Lindungi lingkungan produksi (Production Guard)
    let is_production = content.lines().any(|l| {
        let trimmed = l.trim();
        (trimmed.starts_with("APP_ENV=production") || trimmed.starts_with("APP_ENV=prod"))
            && !trimmed.starts_with('#')
    });

    if is_production && !force {
        return Err(
            "Aplikasi terdeteksi berada di lingkungan produksi (APP_ENV=production)! \
             Gunakan flag `--force` jika Anda benar-benar yakin ingin mengganti kunci enkripsi aplikasi."
                .to_string(),
        );
    }

    // 4. Perbarui baris APP_KEY
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut app_key_found = false;

    for line in lines.iter_mut() {
        let trimmed = line.trim();
        if trimmed.starts_with("APP_KEY=") && !trimmed.starts_with('#') {
            *line = format!("APP_KEY={}", key);
            app_key_found = true;
            break;
        }
    }

    if !app_key_found {
        lines.push(format!("APP_KEY={}", key));
    }

    // 5. Perbarui JWT_SECRET jika diminta
    if let Some(jwt) = jwt_key {
        let mut jwt_found = false;
        for line in lines.iter_mut() {
            let trimmed = line.trim();
            if trimmed.starts_with("JWT_SECRET=") && !trimmed.starts_with('#') {
                *line = format!("JWT_SECRET={}", jwt);
                jwt_found = true;
                break;
            }
        }
        if !jwt_found {
            lines.push(format!("JWT_SECRET={}", jwt));
        }
    }

    // 6. Simpan kembali berkas .env
    let mut new_content = lines.join("\n");
    if !new_content.ends_with('\n') {
        new_content.push('\n');
    }

    fs::write(env_path, new_content)
        .map_err(|e| format!("Gagal menulis berkas {}: {}", env_path.display(), e))?;

    Ok(())
}

/// Eksekusi logika utama perintah `canting key:generate`
pub fn run_key_generate(
    show: bool,
    force: bool,
    env_file: &str,
    cipher: &str,
    with_jwt: bool,
) -> Result<String, String> {
    let key = generate_key_string(cipher);

    // Jika mode --show, hanya tampilkan kunci tanpa memodifikasi berkas
    if show {
        println!("{}", key);
        return Ok(key);
    }

    let env_path = Path::new(env_file);
    let jwt_key = if with_jwt {
        Some(generate_key_string("hex"))
    } else {
        None
    };

    set_key_in_env_file(env_path, &key, jwt_key.as_deref(), force)?;

    println!(
        "{} Application key [{}] set successfully.",
        "✔".bright_green().bold(),
        key.bright_cyan().bold()
    );

    if let Some(ref jwt) = jwt_key {
        println!(
            "{} JWT Secret [{}] set successfully.",
            "✔".bright_green().bold(),
            jwt.bright_yellow().bold()
        );
    }

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_generate_base64_key() {
        let key = generate_key_string("base64");
        assert!(key.starts_with("base64:"));
        let b64_part = &key["base64:".len()..];
        // 32 byte di-encode base64 menghasilkan 44 karakter (dengan padding)
        assert_eq!(b64_part.len(), 44);
    }

    #[test]
    fn test_generate_hex_key() {
        let key = generate_key_string("hex");
        assert_eq!(key.len(), 64);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_set_key_in_env_file_replaces_existing() {
        let temp_dir = env::temp_dir().join(format!("canting_key_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let env_file = temp_dir.join(".env");

        let initial_env = "APP_NAME=Pakem\nAPP_KEY=old_secret_key\nAPP_DEBUG=true\n";
        fs::write(&env_file, initial_env).unwrap();

        let new_key = "base64:AbCdEf1234567890AbCdEf1234567890AbCdEf12345=";
        set_key_in_env_file(&env_file, new_key, None, false).unwrap();

        let updated = fs::read_to_string(&env_file).unwrap();
        assert!(updated.contains(&format!("APP_KEY={}", new_key)));
        assert!(!updated.contains("old_secret_key"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_set_key_in_env_file_appends_when_missing() {
        let temp_dir = env::temp_dir().join(format!("canting_key_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let env_file = temp_dir.join(".env");

        let initial_env = "APP_NAME=Pakem\nAPP_DEBUG=true\n";
        fs::write(&env_file, initial_env).unwrap();

        let new_key = "base64:FreshGeneratedKeyForTesting1234567890123=";
        set_key_in_env_file(&env_file, new_key, None, false).unwrap();

        let updated = fs::read_to_string(&env_file).unwrap();
        assert!(updated.contains(&format!("APP_KEY={}", new_key)));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_production_guard_requires_force() {
        let temp_dir = env::temp_dir().join(format!("canting_key_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let env_file = temp_dir.join(".env");

        let initial_env = "APP_NAME=Pakem\nAPP_ENV=production\nAPP_KEY=some_prod_key\n";
        fs::write(&env_file, initial_env).unwrap();

        let res = set_key_in_env_file(&env_file, "base64:new_key", None, false);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("produksi"));

        // Harus berhasil jika --force aktif
        let res_forced = set_key_in_env_file(&env_file, "base64:forced_key", None, true);
        assert!(res_forced.is_ok());
        let updated = fs::read_to_string(&env_file).unwrap();
        assert!(updated.contains("APP_KEY=base64:forced_key"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_set_jwt_secret_alongside_app_key() {
        let temp_dir = env::temp_dir().join(format!("canting_key_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let env_file = temp_dir.join(".env");

        let initial_env = "APP_KEY=old_app_key\nJWT_SECRET=old_jwt_secret\n";
        fs::write(&env_file, initial_env).unwrap();

        set_key_in_env_file(&env_file, "base64:new_app_key", Some("new_jwt_hex_secret"), false).unwrap();

        let updated = fs::read_to_string(&env_file).unwrap();
        assert!(updated.contains("APP_KEY=base64:new_app_key"));
        assert!(updated.contains("JWT_SECRET=new_jwt_hex_secret"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
