# svd-lint

Статический анализатор CMSIS-SVD файлов. Реализованы **Stage 1–4**:
фронтенд разбора, Canonical IR, нормализация, семантические проверки и CLI
с точными исходными позициями. Stage 4 добавляет профили, corpus, batch scan,
SARIF и опциональный XSD backend.

## Структура

```text
crates/
├── svd-lint-core/   # библиотека: SourceFile, SourceMap, preflight, parse_svd, диагностики
└── svd-lint-cli/    # бинарник svd-lint (clap + miette + JSON)
tests/fixtures/      # общие SVD-фикстуры (valid/ и invalid/)
```

`svd-lint-core` не зависит от CLI и не зависит от `miette` — модель
диагностик независима от рендеринга, сериализуется через `serde`.

## Использование

```console
$ svd-lint check path/to/device.svd
$ svd-lint check path/to/device.svd --format json
$ svd-lint dump-ir path/to/device.svd --format json
```

- `--format text` (по умолчанию) — читаемые диагностики с кодом, путём,
  номером строки/колонки и подсветкой фрагмента (miette), вывод в stderr.
- `--format json` — стабильный JSON-отчёт в stdout: имя файла, список
  диагностик с байтовыми диапазонами `[start, end)` и вычисленными
  строками/колонками (1-based, колонки — в символах Unicode).

### Коды выхода

| Код | Значение |
|-----|----------|
| 0   | ошибок нет (предупреждения допускаются) |
| 1   | в SVD обнаружены ошибки |
| 2   | файл невозможно прочитать или анализ невозможно начать |

### Каталог правил

Стабильные коды, severity, происхождение проверок, примеры и ссылки на
спецификацию: [генерируемый каталог](docs/rules.md).
Соответствия внешней диагностике: [SVDConv coverage](docs/svdconv-coverage.md).

```console
svd-lint list-rules --format json
svd-lint explain SEM004
```

## Что делает Stage 1

1. Проверяет синтаксис XML (`roxmltree`); при ошибке — одна диагностика
   `XML001` с честной позицией, дальнейший разбор прекращается.
2. Preflight: корневой `<device>`, обязательные одиночные дочерние
   элементы (`name`, `version`, `description`, `addressUnitBits`, `width`,
   `peripherals`), их дубликаты и пустые значения, наличие хотя бы одного
   `<peripheral>`, предупреждение о периферии без `<description>`.
   Все ошибки собираются за один проход, без дублирования.
3. Типизированный разбор через `svd_parser::parse_with_config`
   (`ValidateLevel::Disabled`, без раскрытия `derivedFrom` и `dim`,
   перечисления разбираются). Ошибки преобразуются в `SVD006`; если позиция
   неизвестна, `primary_span = None` — координаты не выдумываются.
4. Строит `SourceMap`: индекс всех XML-элементов с диапазонами элементов и
   атрибутов (отдельно имени и значения), привязанный к `SourceId`.
   `NodeId` двух независимых разборов не эквивалентны; заимствованные
   `roxmltree::Node` в долгоживущих структурах не хранятся.

## Публичный API библиотеки

```rust
use svd_lint_core::{parse_svd, SourceFile, SourceMap};

let source = SourceFile::new("device.svd", text);
let result = parse_svd(&source);
// result.device:      Option<svd_parser::svd::Device>
// result.diagnostics: Vec<Diagnostic>
// result.source_map:  Option<SourceMap>

let pos = source.line_col(byte_offset); // Option<LineCol> (1-based)
```

## Разработка

```console
$ cargo build --workspace
$ cargo test --workspace
$ cargo fmt --all -- --check
$ cargo clippy --workspace --all-targets -- -D warnings
```

Snapshot-тесты используют `insta`; обновление снапшотов:
`INSTA_UPDATE=always cargo test --workspace`.

## Stage 2: Canonical IR и нормализация

`check` выполняет разбор, нормализацию и семантический анализ. `dump-ir` выводит
полную Canonical IR в JSON, если нормализация завершилась успешно. Ошибки разбора
или нормализации не позволяют получить IR. Семантические ошибки сохраняют IR:
`dump-ir` выводит её в stdout, диагностики — в stderr и возвращает код 1.
Форматы JSON для IR и диагностик сохранены; диагностики не включаются в IR.

```rust
use svd_lint_core::{analyze_svd, normalize, NormalizeConfig, SourceFile};

let source = SourceFile::new("device.svd", text);
let analysis = analyze_svd(&source);
// analysis.device: Option<CanonicalDevice>, только полная IR
// analysis.diagnostics: диагностики всех трёх этапов
// analysis.source_map: карта для разрешения Origin::declaration и usage

// Для отдельно полученного результата parse_svd:
let config = NormalizeConfig { max_instances: 100_000 };
// normalize(&device, &source_map, &config);
```

Поддержаны `derivedFrom` для peripheral, cluster, register, field и
enumeratedValues, ссылки вперёд и между областями видимости, вложенные
кластеры и массивы всех четырёх видов. `Resolved<T>` сохраняет источник
эффективных свойств, `Origin` — XML-узлы объявления и использования,
цепочку наследования и все измерения вложенного массива.

Размеры, доступ, защита и reset-свойства вычисляются после разрешения
`derivedFrom`, от Device к Register; поля наследуют доступ регистра.
Неуказанные необязательные свойства остаются `None`, значения не выдумываются.
Размер регистра обязателен для Canonical IR. Метаданные альтернативных
представлений, прерывания, addressBlock и эффекты чтения/записи сохраняются.

Порядок JSON соответствует порядку объявлений и индексов. Адреса остаются
в единицах адресного пространства SVD, `address_unit_bits` сохраняется.
Лимит `max_instances` общий для периферии, кластеров, регистров и полей;
он проверяется до выделения памяти для раскрываемого массива.

Подробности API, тестов и ограничений: [отчёт Stage 2](docs/stage2.md).

## Stage 3: семантический анализ

Независимый Rule Engine проверяет неизменяемую Canonical IR после нормализации.
Все SEM001–SEM016 собираются за один запуск. Проверки учитывают эффективные
свойства, происхождение унаследованных значений и экземпляры массивов.

Адресные диапазоны вычисляются в единицах `addressUnitBits`, включая размеры
регистров, не кратные адресной единице. Поддержаны несколько раздельных
`addressBlock`, альтернативные регистры, группы и кластеры. Одинаковый адрес
без подтверждённой альтернативы даёт предупреждение; доказанное частичное
пересечение без альтернативных метаданных — ошибку. Между разными перифериями
пересечения не диагностируются.

Наборы перечислений проверяются отдельно для чтения и записи. Шаблоны с `x`
сохраняются в SourceMap, чтобы не спутать их с обычными числовыми значениями.
Повторные ошибки одной декларации массива объединяются; нарушения по разным
физическим адресам сохраняются. Конфликтующие элементы указываются в `related`.

```rust
use svd_lint_core::check_semantics;

// Самостоятельная проверка уже полученной IR:
// let diagnostics = check_semantics(&device, &source_map);
```

Перечень проверок, тесты и ограничения: [отчёт Stage 3](docs/stage3.md).

## Stage 4: профили и tooling

```console
svd-lint check device.svd --profile strict --fail-on warning
svd-lint scan ./packs --recursive --exclude "**/legacy/*.svd" --format json
svd-lint scan a.svd b.svd --format sarif
svd-lint check device.svd --format sarif --config svd-lint.toml
```

Профиль `default` сохраняет диагностики Stage 3. `strict` добавляет проверку
качества описаний регистров. `svdconv` включает эту проверку и повышает
SEM004/SEM013 до Error; неоднозначные адресные наложения остаются Warning.

Конфигурация `svd-lint.toml` загружается из рабочего каталога либо через
`--config`. `--profile` имеет приоритет над профилем файла. Для lint-правил
поддержаны `enabled`, severity `allow|warning|error` и файловые overrides с glob.
Ошибки, препятствующие анализу, нельзя отключать или понижать до warning.

```toml
profile = "default"

[rules.SEM004]
severity = "error"

[[overrides]]
files = ["vendor/*.svd"]
rules = ["SEM004"]
severity = "allow"
```

JSON команды `check` сохранён. У `scan` отдельный формат версии 1 с отсортированными
файлами, диагностикой и summary. Файлы обрабатываются последовательно; после
каждого освобождаются XML и IR. Директории обходятся без перехода по вложенным
symlink. SARIF 2.1.0 включает каталог правил, основные и связанные позиции;
неизвестные координаты не подставляются. `--fail-on warning` возвращает 1 и
при предупреждениях во всех форматах; значение по умолчанию — `error`.

XSD используется только по запросу и при сборке с feature:

```console
cargo build -p svd-lint-cli --features xsd
svd-lint check device.svd --xsd --schema-dir schemas --schema-version 1.3.10
```

Проверка XSD предшествует typed parsing. Схемы не включены в репозиторий:
их лицензии проверяются по файлам, внешний каталог предоставляется явно.
Маркер `schemaVersion="1.3"` выбирает 1.3.12 из pinned набора; явная patch-версия
выбирается точно. Результаты допуска backend: [XSD spike](docs/xsd-spike.md).

```console
cargo xtask test-corpus
cargo xtask generate-docs
cargo xtask generate-docs --check
cargo xtask fetch-conformance
cargo xtask test-conformance --suite SVDSuite --limit 0
cargo xtask test-conformance --suite data --limit 100
```

Обычные workspace tests не обращаются к сети. Для внешних тестов требуется
отдельная Python-среда с [reference dependencies](tools/conformance-requirements.txt)
и pinned SVDSuite. Подробности запуска, отчёты и ограничения:
[Stage 4](docs/stage4.md), [external corpus](tests/external/README.md).
Fuzz targets вынесены в [fuzz](fuzz/README.md).

Автоисправления, генерация SVD/PAC/headers, LSP, GUI и Lean остаются для
последующих проектов.
