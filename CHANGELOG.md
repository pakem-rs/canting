# 📝 Changelog — `canting`

Semua perubahan penting pada perkakas CLI **Canting** (`canting`) didokumentasikan di dalam berkas ini.

Format pencatatan mengacu pada [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) dan mengikuti standar [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.0.5] - 2026-11-27

### ⚡ Interactive Toolchain Upgrade & Dependency Updater
- **Penyempurnaan `canting upgrade` (CLI Self-Update)**:
  - Header visual elegan dengan informasi status versi saat ini, target rilis, dan sumber instalasi.
  - Streaming progress real-time yang menampilkan proses pengunduhan crate (`⬇ Downloading <crate>`), kompilasi unit (`⚙ Building`), dan linking binary.
  - Ringkasan statistik akhir dalam kotak status visual: total crate diunduh, unit yang dikompilasi, dan durasi eksekusi dalam detik.
- **Penyempurnaan `canting update` (Modern Composer-like UX)**:
  - Tampilan visual pembaruan dependensi ala `composer update`: `↑ Upgrading <pkg> vA ➜ vB`, `+ Installing`, `- Removing`, dan `↓ Downgrading`.
  - Box ringkasan lockfile: jumlah paket yang di-upgrade, diinstal baru, dan dihapus.
  - Daftar rincian 12 paket utama yang di-upgrade dengan visual panah modern `➜`.
  - Dukungan filter `--dry-run`, `--pakem-only`, dan pembaruan spesifik package (`canting update <pkg>`).

### 🦀 Rust 2024 Edition & Axum 0.8 Stack
- Migrasi manifest ke Rust 2024 Edition (`edition = "2024"`).
- Sinkronisasi dependensi dan integrasi runtime web Axum 0.8.
- Pembersihan tag author personal dari manifest Cargo.

---

## [1.0.4] - 2026-11-26

### 🔑 Application Key Generator (`canting key:generate`)
- Menambahkan perintah `canting key:generate` (alias: `pakem key:generate`) untuk membuat `APP_KEY` 256-bit berbasis CSPRNG.
- Flag `--show`, `--force`, `--env <path>`, dan `--jwt` untuk sinkronisasi `JWT_SECRET`.
- Auto-generate `APP_KEY` saat scaffolding proyek baru (`canting new`).
