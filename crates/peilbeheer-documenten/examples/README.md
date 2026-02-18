# PDF Batch Parser

Parse alle PDF bestanden in een directory en exporteer tekst en metadata.

## Gebruik

### 1. Plaats PDF's in een directory

```bash
mkdir peilbesluiten
cp ~/Downloads/*.pdf peilbesluiten/
```

### 2. Run de batch parser

```bash
# Default directory (peilbesluiten)
cargo run --example batch_parse

# Custom directory
cargo run --example batch_parse -- /pad/naar/pdf/dir
```

### 3. Output

De parser maakt twee bestanden:

- **peilbesluiten_data.json** - Gestructureerde data met metadata
- **peilbesluiten_text.txt** - Alle tekst in één bestand

## Voorbeeld output

```
=== PDF Batch Parser ===
Directory: peilbesluiten

3 PDF bestanden gevonden:

[1/3] Parsing: peilbesluit-001.pdf
  ✓ Titel: Peilbesluit waterstand Waal
  ✓ Auteur: Waterschap Rijnland
  ✓ Tekst lengte: 4523 karakters
  ✓ Chunks: 5

[2/3] Parsing: peilbesluit-002.pdf
  ✓ Titel: Keuring gemaal Albert
  ✓ Auteur: Waterschap Rijnland
  ✓ Tekst lengte: 2891 karakters
  ✓ Chunks: 3

...
```

## JSON formaat

```json
[
  {
    "filename": "peilbesluit-001.pdf",
    "title": "Peilbesluit waterstand Waal",
    "author": "Waterschap Rijnland",
    "text_length": 4523,
    "chunk_count": 5,
    "full_text": "Volledige tekst..."
  }
]
```
