# Nexa

**A fast, native Windows media library manager built with Rust and egui.**

Nexa is a desktop application for organizing, browsing, and distributing large collections of images and videos. It is
designed to outperform Windows File Explorer in responsiveness and resource efficiency when working with libraries of
50,000+ files.

---

## Features

### Library Management

- **Grid view** with virtualized rendering — only visible cards are drawn, keeping frame times low regardless of library
  size
- **Natural sort order** — filenames with numbers sort as humans expect (`file2` before `file10`)
- **Full-text search** via SQLite FTS5 — searches across name, copyright, artist, characters, and tags with prefix
  matching
- **Filter by type** — All / Images / Videos
- **Sort by** name (A→Z, Z→A) or date modified (newest/oldest first)
- **Field filters** — click any artist, copyright holder, or tag in the sidebar to instantly filter the view
- **Multi-select** with rubber-band drag selection and Ctrl+click toggle
- **Bulk delete** — move multiple files to the system Trash in one action

### Metadata

- **Per-file metadata editing** — copyright, artist, characters (multi-value), tags (multi-value)
- **Autocomplete** on all fields, populated from your existing library
- **Pipe-separated storage** for multi-value fields, transparently parsed in the UI
- **Characters extracted from filenames** automatically at scan time using a configurable separator (e.g. `" x "`)
- **Copyright and artist inferred from folder structure** — configurable depth per field

### Staging / Inbox

- **Staging folder** — a separate watched inbox for unsorted incoming files
- **Distribute workflow** — move files from staging into the library with full metadata assignment in one modal
- **Bulk distribute** — select multiple staging items and distribute them in sequence, carrying metadata forward between
  items
- **Auto filename generation** — destination filename is built from characters, artist, and optional video title
- **Conflict resolution** — existing files are automatically renamed to `name - 1`, `name - 2`, etc.

### File Operations

- **File watcher** — library changes made outside the app (new files, renames, deletes) are reflected automatically with
  500 ms debounce
- **Reveal in Explorer** — right-click any file to open its containing folder in Windows Explorer
- **Open file** — double-click or right-click → Open to launch the file in its default application
- **Reorder group** — drag-and-drop reordering for numbered file groups (e.g. `image - 1`, `image - 2`), with atomic
  two-phase rename to prevent conflicts
- **Copy path / filename** to clipboard from the context menu

### Thumbnails & Previews

- **Windows Shell thumbnail extraction** via `IShellItemImageFactory` — uses the same source as Explorer, supporting all
  formats Explorer supports
- **WebP thumbnail cache** — generated thumbs are stored as lossy WebP files, keyed by path + mtime hash
- **LRU eviction** — cache is capped (600 textures in VRAM) with automatic eviction of oldest entries
- **Background worker pool** — 2–4 threads decode and upload thumbnails off the UI thread
- **Prefetch** — rows adjacent to the viewport are pre-loaded before they scroll into view
- **Cache pruning** — background thread trims the on-disk cache to 500 MB on startup

### Search

- **SQLite FTS5** full-text index covering name, copyright, artist, characters, and tags
- **Prefix matching** — every search token is treated as a prefix query (`"foo"*`)
- **Combined filters** — FTS results are intersected with type filters and field filters in a single query
- **Debounced input** — search fires 300 ms after the user stops typing

### Auto-Update

- **GitHub Releases integration** — checks for newer versions against the configured repo on startup
- **In-app download** with a progress bar and cancel support
- **Self-replacing update** — spawns a detached helper process that waits for the main process to exit, then atomically
  replaces the executable and restarts

### Settings

- Library folder and staging folder selection
- Folder mapping configuration (which depth is copyright, which is artist)
- Character separator for filename parsing
- Video subfolder name (e.g. `Videos/` inside each artist folder)
- Card size slider (120–320 px)
- Toggle thumbnail previews on/off
- Auto-scan on startup toggle
- Thumbnail cache usage display with one-click clear
- Auto-update check toggle

### Window & Platform

- **Frameless window** with custom title bar (minimize, maximize/restore, close)
- **Windows DWM integration** — rounded corners, drop shadow, system backdrop (Mica/Acrylic where available)
- **Resize handles** implemented via `WM_NCHITTEST` subclassing
- **Singleton enforcement** — a second launch focuses the existing window instead of opening a duplicate
- **Custom Inter font** embedded in the binary

---

## Requirements

- **Windows 10 or 11** (Windows Shell thumbnail API required)
- No runtime dependencies — the binary is fully self-contained

---

## Installation

Download `Nexa.exe` from the [latest release](../../releases/latest) and run it. No installer required.

---

## Configuration

All settings are stored as JSON at:

```
%LOCALAPPDATA%\%USERNAME%\Nexa\config\settings.json
```

The database is stored at:

```
%LOCALAPPDATA%\%USERNAME%\Nexa\db\vault.db
```

Thumbnails are cached at:

```
%LOCALAPPDATA%\%USERNAME%\Nexa\cache\thumbnails\
```

---

## Folder Structure Convention

Nexa infers metadata from your folder hierarchy. The default mapping is:

```
<library root>/
└── <copyright>/        ← depth 0  (e.g. "Marvel", "Studio Ghibli")
    └── <artist>/       ← depth 1  (e.g. "John Doe")
        └── image.jpg
```

Both depths are configurable per-library in Settings → Structure.

### Filename conventions

Characters are extracted from the filename stem before the artist bracket:

```
CharacterA x CharacterB [ArtistName].jpg
│─────────────────────┘  │──────────┘
     characters              artist
```

The separator (`" x "` by default) and the artist bracket format (`[name]`) are both configurable.

---

## Database Schema

Nexa uses SQLite with WAL mode and a custom `NATURALSORT` collation. The schema is managed through sequential
migrations (currently at version 10).

Key tables:

| Table            | Purpose                                                         |
|------------------|-----------------------------------------------------------------|
| `media`          | Main library — one row per file                                 |
| `media_fts`      | FTS5 virtual table mirroring `media`, kept in sync via triggers |
| `staging`        | Inbox files not yet distributed to the library                  |
| `schema_version` | Single-row migration tracker                                    |

---

## License

This project is licensed under the MIT License - see the LICENSE file for details.