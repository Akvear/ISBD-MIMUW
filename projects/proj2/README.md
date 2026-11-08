# klakier

Format pliku (`.klk`) inspirowany Parquetem.

## Uruchomienie

```sh
make                                              # cargo build --release
make test                                         # cargo test

cargo run --release --bin gen -- dane.klk 10000   # losowy plik: 1× INT64, 1× BOOLEAN, 2× VARCHAR
cargo run --release --bin stats -- przyklad.klk   # metryki dla każdej kolumny
```

`stats` liczy średnią dla INT64, entropię binarną dla BOOLEAN i liczbę znaków
ASCII dla VARCHAR. Przykładowy plik: `przyklad.klk` (1 mln wierszy).

## Format pliku

```text
"KLAK"                     magic
row group 0                po jednym chunku na kolumnę
row group 1 ...
footer                     schemat + offset i długość każdego chunka
footer_len (u32 LE)
"KLAK"                     magic
```

Kodowanie kolumn:

| Typ     | Kodowanie                                               |
|---------|---------------------------------------------------------|
| INT64   | delta → zigzag → varint                                 |
| BOOLEAN | 1 bit na wartość                                        |
| VARCHAR | długości jako varint + wszystkie napisy w jednej ramce zstd |

Footer jest na końcu, więc writer zapisuje dane strumieniowo, a reader czyta
row groupy po jednym.

## Pamięć

Dane w pamięci są odczytywane do `Batch` (co najwyżej 1024 wierszy) w
układzie kolumnowym:

- `Int64(Vec<i64>)`, `Bool(Vec<bool>)`: ciągłe data slice,
- `Varchar`: wszystkie napisy w jednym buforze `data` + tablica `offsets`.

## Struktura

```text
src/
  batch.rs, column.rs, schema.rs   reprezentacja w pamięci
  encoding/                        kodowanie kolumn
  file/                            FileWriter, FileReader, footer
  bin/gen.rs, bin/stats.rs         programy
tests/                             testy end-to-end na plikach
```

## Testy

Są dwa rodzaje testów:

- **unit testy**: sprawdza kodowanie i dekodowanie wszystkich typów danych, footera i batchy
- **end-to-end** (`tests/`): zapis do prawdziwego pliku i odczyt z powrotem
  (`roundtrip.rs`) oraz uszkodzone pliki, które powinny zwracać błąd (`corrupted.rs`).
