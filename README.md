# ⚡ Canting CLI (`canting`)

<p align="center">
  <img src="https://raw.githubusercontent.com/pakem-rs/pakem/1.x/resources/images/logo.svg" alt="Pakem Framework Logo" width="120">
</p>

> **"Look like Laravel, Run at Rust Speed."**  
> *The official Artisan-like command-line toolchain for Pakem Framework.*

[![License: MIT](https://img.shields.io/badge/License-MIT-emerald.svg)](LICENSE)
[![Rust 2024](https://img.shields.io/badge/Rust-2024_Edition-orange.svg)](https://www.rust-lang.org)
[![Pakem Framework](https://img.shields.io/badge/ecosystem-Pakem%20Framework-orange.svg)](https://github.com/pakem-rs/framework)

---

## 🌟 Tentang Canting CLI

`canting` menghadirkan keajaiban pengalaman pengembang (*Developer Experience*) ala **Laravel Artisan** ke dalam dunia **Rust**. Dirancang untuk membangkitkan proyek, merancang boilerplate secara otomatis (*scaffolding*), mengeksekusi migrasi database, mengelola antrean latar belakang (*background queue*), hingga menjalankan audit keamanan proyek secara instan.

---

## 📦 Instalasi

### Dari Crates.io (Rekomendasi)
```bash
cargo install canting
```

### Dari GitHub Resmi
```bash
cargo install --git https://github.com/pakem-rs/canting.git --force
```

### Atau Build dari Sumber
```bash
git clone https://github.com/pakem-rs/canting.git
cd canting
cargo install --path .
```

Verifikasi instalasi dengan mengetik:
```bash
canting --version
```

---

## 🚀 Panduan Penggunaan & Perintah Lengkap

### 1. Inisialisasi Proyek Baru
Membangkitkan struktur aplikasi Pakem lengkap dengan pilihan database:
```bash
# Proyek baru dengan database SQLite (default)
canting new my_app

# Proyek baru dengan database PostgreSQL
canting new my_app --db postgres

# Proyek baru dengan database MySQL
canting new my_app --db mysql

# Mode offline (menggunakan embedded template)
canting new my_app --offline
```

---

### 2. Menjalankan Server & Pengembangan
```bash
# Menjalankan server aplikasi (cargo run)
canting serve

# Menjalankan server dengan live-reload otomatis (cargo watch)
canting watch
```

---

### 3. Artisan-Style Generators (`make:*`)

Bebaskan diri Anda dari penulisan *boilerplate* manual:

```bash
# Membangkitkan Controller baru
canting make:controller UserController

# Membangkitkan Model ORM + Berkas Migrasi otomatis
canting make:model Product

# Membangkitkan berkas migrasi database kosong
canting make:migration create_orders_table

# Membangkitkan Full CRUD instan (Model, Controller, Migrasi, Blade Views)
canting make:crud Post title:string content:text is_published:boolean

# Membangkitkan sistem Autentikasi lengkap (Register, Login, Session, Model User)
canting make:auth

# Membangkitkan Model Factory & Faker untuk unit testing dan seeding
canting make:factory UserFactory

# Membangkitkan template view Blade Native
canting make:view dashboard.analytics

# Membangkitkan Background Queue Job
canting make:job SendInvoiceEmail

# Membangkitkan Komponen Reaktif Pakem Live (Server-Driven Reactive UI)
canting make:live Counter
```

---

### 4. Database Migration Engine
Eksekusi migrasi database transaksional tanpa memerlukan dependensi eksternal:
```bash
# Jalankan semua migrasi pending
canting migrate

# Batalkan (rollback) batch migrasi terakhir
canting migrate:rollback
```

---

### 5. Background Queue Worker
Jalankan pemrosesan antrean asinkron untuk email, notifikasi, dan kalkulasi berat:
```bash
# Menjalankan worker pada queue default
canting queue:work

# Menjalankan worker pada antrean khusus
canting queue:work --queue high-priority
```

---

### 6. Keamanan & Pemeliharaan Proyek
```bash
# Pakem Shield: Audit keamanan otomatis untuk file .env, dependensi, dan secret leak
canting audit

# Membersihkan cache kompilasi compiler & build artifacts (membebaskan ruang disk)
canting clean
canting clean --purge

# Menghasilkan APP_KEY 256-bit (opsi --jwt, --show, --force)
canting key:generate

# Memperbarui dependensi proyek dengan ringkasan diff ala Composer
canting update
canting update --pakem-only
canting update --dry-run

# Memperbarui binary toolchain Canting CLI
canting upgrade
```

---

## 📋 Ringkasan Perintah

| Perintah | Deskripsi |
| :--- | :--- |
| `canting new <name>` | Membangkitkan skeleton aplikasi baru (*batteries-included*) |
| `canting serve` | Menjalankan server aplikasi di port 8000 |
| `canting watch` | Server dengan hot-reloading otomatis saat kode berubah |
| `canting make:controller` | Membuat controller web/API |
| `canting make:model` | Membuat model ORM `PakemModel` |
| `canting make:crud` | Membangkitkan seluruh alur CRUD dalam 1 detik |
| `canting make:auth` | Sistem autentikasi instan (Login, Register, Session) |
| `canting make:live` | Komponen reaktif fullstack Pakem Live |
| `canting make:factory` | Model Factory & Faker Indonesia |
| `canting migrate` | Menjalankan migrasi database SQLx |
| `canting migrate:rollback`| Rollback migrasi database terakhir |
| `canting queue:work` | Menjalankan background queue consumer |
| `canting audit` | Pengecekan audit keamanan konfigurasi |
| `canting key:generate` | Menghasilkan `APP_KEY` 256-bit & opsi `--jwt` |
| `canting aurora:install` | Memasang starter kit **Pakem Aurora** |
| `canting clone:laravel\|ci\|api\|db` | Polyglot cloner dari Laravel, CI, OpenAPI, atau DB live |
| `canting tinker` | REPL interaktif untuk model & query database |
| `canting clean` | Membersihkan cache build target yang membengkak |
| `canting update` | Memperbarui dependensi proyek (`--pakem-only`, `--dry-run`) |
| `canting upgrade` | Memperbarui CLI ke versi termutakhir |

---

## 🤝 Berkontribusi (Contributing)

Kontribusi dari komunitas sangat disambut dengan antusias!
1. Fork repositori ini.
2. Buat branch fitur baru (`git checkout -b feature/nama-fitur`).
3. Commit perubahan Anda (`git commit -m 'feat: tambah perintah baru'`).
4. Push ke branch Anda (`git push origin feature/nama-fitur`).
5. Buat **Pull Request**.

---

## 📜 Lisensi

Canting CLI dirilis di bawah lisensi terbuka [MIT License](LICENSE).  
Dikelola dengan bangga oleh tim pengembang [Pakem Framework](https://github.com/pakem-rs/framework).

