# Sprawozdanie z Laboratorium 1: latencja i przepustowość odczytów

## Środowisko

| | |
|---|---|
| Komputer | 16 rdzeni, 15 GiB RAM |
| Dysk | NVMe Intel SSDPEKNU512GZ |
| Plik testowy | `testfile.bin`, 1 GiB (1 073 741 824 B) |

## Metodyka

Każda konfiguracja była uruchamiana 5 razy, a w tabelach podane jest minimum z pięciu pomiarów. Czas mierzą same programy (`std::time::Instant`).

Warianty pamięci podręcznej plików:

- **cold** (bez file cache): przed każdym pomiarem `vm.drop_caches=3`, więc plik jest czytany z dysku,
- **warm** (z file cache): jedno nieliczone uruchomienie rozgrzewające, potem pomiary,
- **256M / 512M**: cały przebieg w grupie cgroup z `MemoryMax`

Poprawność sprawdzana była przez zgodność wyniku: dla danego `M` (i `K`) wszystkie programy, liczby wątków i warianty dają ten sam hash.

---

## 1. Latencja

Wersja poprawiona: `T` wątków czytających wykonuje `read_exact_at` z wcześniej wylosowanymi offsetami (wspólny licznik `AtomicUsize`). Bloki trafiają przez `sync_channel` do jednego wątku liczącego MD5 (`K = 1`).

### Czas działania [s] w funkcji `M` i liczby wątków `T` – bez limitu pamięci, bez file cache

| M \ T | 1 | 2 | 4 | 8 | 16 | 32 | 64 |
|---|---|---|---|---|---|---|---|
| 512 B | 32,22 | 20,90 | 16,45 | 12,66 | 11,91 | **11,67** | 11,95 |
| 4 KiB | 26,21 | 14,91 | 9,66 | 7,38 | 6,39 | 6,07 | **5,96** |
| 64 KiB | 4,81 | 2,36 | **1,61** | 1,64 | 1,65 | 1,68 | 1,69 |
| 1 MiB | 1,68 | **1,60** | 1,62 | 1,59 | 1,59 | 1,62 | 1,62 |

### Ten sam przebieg z limitem 256M

| M \ T | 1 | 2 | 4 | 8 | 16 | 32 | 64 |
|---|---|---|---|---|---|---|---|
| 512 B | 247,70 | 106,71 | 66,99 | 48,54 | 40,11 | 37,19 | **35,74** |
| 4 KiB | 27,29 | 14,66 | 9,44 | 6,90 | 5,80 | 5,45 | **5,30** |
| 64 KiB | 3,51 | 2,00 | **1,62** | 1,62 | 1,63 | 1,63 | 1,63 |
| 1 MiB | 1,60 | 1,60 | 1,61 | 1,61 | 1,62 | 1,64 | OOM\* |

\*Konfiguracja `M = 1 MiB, T = 64` została zabita przez OOM killera z powodu wyczerpania pamięci.

### Ten sam przebieg z plikiem w file cache

| M \ T | 1 | 2 | 4 | 8 | 16 | 32 | 64 |
|---|---|---|---|---|---|---|---|
| 512 B | **3,10** | 3,57 | 4,70 | 3,95 | 4,18 | 4,21 | 4,28 |
| 4 KiB | 1,92 | 1,86 | **1,86** | 1,87 | 1,88 | 1,89 | 1,89 |
| 64 KiB | **1,57** | 1,59 | 1,59 | 1,59 | 1,59 | 1,59 | 1,59 |
| 1 MiB | 1,56 | 1,56 | **1,55** | 1,56 | 1,57 | 1,57 | 1,58 |

### Wnioski

**Latencja pojedynczego odczytu z dysku.** Dla `T = 1` czas ≈ N · latencja:

- `M = 4 KiB`: 26,21s / 262 144 odczytów ≈ **100µs na odczyt**, typowa latencja losowego odczytu NVMe.
- `M = 512 B` z limitem 256M: 247,7s / 2 097 152 ≈ **118µs na odczyt**. Każdy 512-bajtowy blok kosztuje praktycznie pełny odczyt strony z dysku.
- `M = 512 B` bez limitu: tylko **15,4µs na odczyt**. Jądro czyta z dysku całe strony 4 KiB, a jedna strona zawiera 8 bloków po 512 B. Bez limitu pamięci strony zostają w file cache i pozostałe bloki tej samej strony, wylosowane później, są czytane z pamięci. Z limitem 256M strony mogą być zapomniane, zanim będą znowu potrzebne, stąd spowolnienie 32,2s → 247,7s.

**Czy więcej wątków skraca czas programu?** Tak, ale tylko do pewnego momentu:

- `M = 4 KiB`: 26,2s → 5,96s (**4,4×**), z nasyceniem od ≈ 16–32 wątków.
- `M = 512 B` z limitem 256M: 247,7s → 35,7s (**6,9×**).
- `M = 64 KiB`: poprawa tylko do `T = 4` (4,81 → 1,61s). Dalej program dochodzi do granicy szybkości MD5 (≈ 1,6s) i więcej wątków nic nie daje.
- `M = 1 MiB`: wątki nie mają znaczenia, bo od początku ograniczeniem jest MD5.

Zwiększenie ilości wątków w pewnym momencie nie ma sensu, ponieważ narzut tworzenia i zwiększona walka o atomica powodują większe straty niż korzyści.

**Czy więcej wątków zmniejsza latencję pojedynczego odczytu?** Nie, latencja rośnie. Dla `M = 4 KiB`:

| T | 1 | 2 | 4 | 16 | 64 |
|---|---|---|---|---|---|
| średnia latencja odczytu | 100µs | 114µs | 147µs | 390µs | 1,45ms |

Wątki nie przyspieszają pojedynczej operacji – pozwalają tylko utrzymywać więcej żądań w kolejce dysku jednocześnie. Średnia latencja została policzona z wzoru: `T · czas / N`, gdzie `N = 1 GiB / 4 KiB`.

**Wpływ file cache na latencję.** Gdy plik jest w pamięci, odczyt nie dotyka dysku. Dla `M = 4 KiB, T = 1` czas spada z 26,2s do 1,92s. Dla małych bloków więcej wątków pogarsza wynik (512 B: 3,10s przy `T = 1`, 4,70s przy `T = 4`). Przy tak małych blokach dominuje stały narzut na każdy blok (wywołanie systemowe, kopia bufora, komunikat w kanale). Dodatkowe wątki czytające zapełniają kanał szybciej, niż wątek liczący go opróżnia, więc ciągle usypiają i są budzone, a ten koszt synchronizacji spowalnia jedyny wątek liczący.

---

## 2. Przepustowość

Wersja bazowa czyta plik sekwencyjnie przez `read()` do bufora o rozmiarze `M`. Wersja poprawiona mapuje cały plik (`mmap`) i liczy skróty bezpośrednio na kolejnych fragmentach `M` zamapowanej pamięci.

### Czas działania [s] – przed (`read`) i po (`mmap`) poprawce

| M | cold: read | cold: mmap | warm: read | warm: mmap | 256M: read | 256M: mmap |
|---|---|---|---|---|---|---|
| 512 B | 3,44 | **1,85** | 3,33 | **1,69** | 3,41 | **2,04** |
| 4 KiB | 1,96 | **1,69** | 1,82 | **1,52** | 1,91 | **1,88** |
| 64 KiB | 1,76 | **1,67** | 1,63 | **1,51** | **1,72** | 1,85 |
| 1 MiB | 2,02 | **1,67** | 1,78 | **1,51** | 1,98 | **1,81** |
| 16 MiB | 2,19 | **1,67** | 1,79 | **1,50** | 2,12 | **1,82** |

### Wnioski

**mmap zawsze pomaga, najbardziej dla małych bloków.** Dla `M = 512 B` czas spada prawie dwukrotnie. Gdy czytane bloki są mniejsze od strony pamięci (4 KiB), `read()` wykonuje kilka wywołań systemowych i kopii na każdą stronę. Przy `mmap` program czyta dane bezpośrednio z file cache, a jądro jest angażowane tylko przy pierwszym dostępie do strony, więc koszt nie zależy od rozmiaru bloku.

**Wersja `read` ma optimum w okolicy 64 KiB.** Dla większych buforów jest wolniejsza. Możliwe, że większe bufory nie mieszczą się w L2 cache'u procesora, dlatego czas wykonania rośnie. `mmap` nie kopiuje, więc rozmiar bloku praktycznie nie ma znaczenia.

**Czy jest schemat odczytu, w którym mmap zmniejszy obciążenie procesora bardziej niż read?**
Tak, odczyt danych, które już są w file cache: mmap eliminuje kopię i wywołania systemowe.

**Wpływ file cache.** Między cold i warm różnica wynosi tylko ≈ 0,15–0,17s. Sekwencyjny odczyt z dysku jest dużo bardziej wydajny niż losowe odczyty. Dla tego programu cache zmienia mało, w przeciwieństwie do odczytu losowego.

---

## 3. Przeplot obliczeń i IO – pipelining

Wersja poprawiona: wątek czytający (`read_exact_at` blokami `M`) i wątek liczący `H(B, K)` (K-krotne MD5) połączone `sync_channel`.

### Czas działania [s] w funkcji `M` i `K` (cold)

| M \ K | 1 | 50 | 100 | 200 | 400 | 800 | 1600 | 3200 | 6400 | 10000 |
|---|---|---|---|---|---|---|---|---|---|---|
| 64 KiB | 1,55 | 1,63 | 1,72 | 1,89 | 2,20 | 2,83 | 4,11 | 6,62 | 11,78 | 17,36 |
| 256 KiB | 1,57 | 1,60 | 1,61 | 1,66 | 1,73 | 1,89 | 2,21 | 2,85 | 4,13 | 5,55 |
| 1 MiB | 1,58 | 1,57 | 1,58 | 1,59 | 1,61 | 1,65 | 1,74 | 1,89 | 2,21 | 2,60 |
| 4 MiB | 1,59 | 1,59 | 1,59 | 1,59 | 1,60 | 1,61 | 1,63 | 1,67 | 1,76 | 1,84 |
| 16 MiB | 1,61 | 1,61 | 1,62 | 1,61 | 1,63 | 1,62 | 1,62 | 1,63 | 1,67 | 1,69 |

### To samo z plikiem w pamięci (warm)

| M \ K | 1 | 50 | 100 | 200 | 400 | 800 | 1600 | 3200 | 6400 | 10000 |
|---|---|---|---|---|---|---|---|---|---|---|
| 64 KiB | 1,51 | 1,59 | 1,67 | 1,84 | 2,16 | 2,77 | 4,11 | 6,54 | 11,58 | 17,21 |
| 256 KiB | 1,55 | 1,56 | 1,59 | 1,64 | 1,70 | 1,87 | 2,16 | 2,79 | 4,05 | 5,49 |
| 1 MiB | 1,56 | 1,55 | 1,56 | 1,56 | 1,58 | 1,62 | 1,69 | 1,82 | 2,14 | 2,51 |
| 4 MiB | 1,54 | 1,54 | 1,55 | 1,55 | 1,57 | 1,56 | 1,58 | 1,62 | 1,70 | 1,80 |
| 16 MiB | 1,60 | 1,60 | 1,60 | 1,60 | 1,60 | 1,61 | 1,61 | 1,62 | 1,64 | 1,66 |

### To samo z limitem 512M

| M \ K | 1 | 50 | 100 | 200 | 400 | 800 | 1600 | 3200 | 6400 | 10000 |
|---|---|---|---|---|---|---|---|---|---|---|
| 64 KiB | 1,52 | 1,64 | 1,75 | 1,92 | 2,25 | 2,87 | 4,12 | 6,69 | 11,71 | 17,57 |
| 256 KiB | 1,58 | 1,59 | 1,61 | 1,66 | 1,74 | 1,90 | 2,23 | 2,87 | 4,13 | 5,55 |
| 1 MiB | 1,58 | 1,59 | 1,59 | 1,60 | 1,62 | 1,66 | 1,75 | 1,90 | 2,22 | 2,61 |
| 4 MiB | 1,62 | 1,62 | 1,63 | 1,65 | 1,61 | 1,63 | 1,64 | 1,68 | 1,76 | 1,87 |
| 16 MiB | 1,63 | 1,63 | 1,63 | 1,64 | 1,63 | 1,63 | 1,64 | 1,65 | 1,67 | 1,69 |

### Obserwacje

- **Czas rośnie z `K` tym szybciej, im mniejsze `M`.** Mniejszy blok oznacza więcej bloków, a więc więcej iteracji MD5 dla tego samego `K`. Przy `K = 10000` program z blokami 64 KiB działa ≈ 11× dłużej niż przy `K = 1`, a z blokami 16 MiB tylko ≈ 5% dłużej.
- **Wyniki cold, warm i z limitem 512M są prawie takie same**. Odczyt jest całkowicie ukryty za obliczeniami. Program nie potrzebuje też file cache: każdy blok jest czytany dokładnie raz, po kolei, więc nawet plik 2× większy od dostępnej pamięci nie spowalnia działania.

### `K_opt` dla `M = 1 MiB`

**`K_opt ≈ 200–400`** zarówno dla pliku w pamięci, jak i czytanego z dysku (również z ograniczoną pamięcią). Jak widać po powyższych tabelkach, różnica czasu między K = 1 i K = 200 mieści się w granicach błędu pomiarowego (rozrzut powtórzeń ≈ 20ms), a przy K = 400 czas rośnie o 1–3%, zależnie od wariantu.

### Czy więcej wątków po stronie czytania lub hashowania coś zmieni?

Nie zostało to bezpośrednio zmierzone, ale pomiary pozwalają to przewidzieć:

- **Więcej wątków czytających przesunie `K_opt` w dół, o ile jeden wątek nie nasyca dysku.** Szybszy odczyt oznacza, że obliczenia wcześniej staną się wąskim gardłem.
- **Więcej wątków liczących przesunie `K_opt` w górę.** Obliczenia dobrze rozłożą się na kilka rdzeni, ponieważ XOR jest łączny i przemienny, więc program wytrzyma większe `K`, zanim zacznie zwalniać.

---

Autor: Jakub Janowski
