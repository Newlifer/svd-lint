# svd-lint

Статический анализатор CMSIS-SVD файлов. Реализованы **Stage 1 и Stage 2**:
фронтенд разбора, Canonical IR, нормализация и CLI с точными исходными позициями.

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

### Коды диагностик

| Код    | Значение |
|--------|----------|
| IO001  | не удалось прочитать файл |
| IO002  | файл не содержит корректный UTF-8 |
| XML001 | синтаксическая ошибка XML |
| SVD001 | некорректный корневой элемент |
| SVD002 | отсутствует обязательный элемент |
| SVD003 | повторяется одиночный элемент |
| SVD004 | обязательный элемент пуст |
| SVD005 | отсутствует описание периферии (warning) |
| SVD006 | ошибка типизированного разбора SVD |
| NORM001 | не найдена ссылка derivedFrom |
| NORM002 | циклическая зависимость derivedFrom |
| NORM003 | неоднозначная ссылка или повторный путь IR |
| NORM004 | некорректная размерность массива |
| NORM005 | некорректные индексы массива |
| NORM006 | некорректный шаблон имени массива |
| NORM007 | переполнение адреса или смещения поля |
| NORM008 | не удалось определить размер регистра |
| NORM009 | превышен лимит материализации экземпляров |

Коды стабильны и не зависят от формулировок сообщений.

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

`check` выполняет оба этапа. `dump-ir` выводит Canonical IR в JSON при успешном
анализе; при ошибке выводит диагностики и возвращает код 1, без частичной IR.
Формат диагностического JSON и коды выхода прежние.

```rust
use svd_lint_core::{analyze_svd, normalize, NormalizeConfig, SourceFile};

let source = SourceFile::new("device.svd", text);
let analysis = analyze_svd(&source);
// analysis.device: Option<CanonicalDevice>, только полная IR
// analysis.diagnostics: диагностики обоих этапов
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

## Последующие этапы

Пересечения адресов и полей, корректность reset-значений и перечислений,
допустимость альтернативных представлений, полная XSD-валидация, SARIF,
автоисправления и Lean пока не реализованы.
