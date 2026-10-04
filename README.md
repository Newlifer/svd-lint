# svd-lint

Статический анализатор CMSIS-SVD файлов. Это **Stage 1**: фронтенд разбора,
система диагностики с точными исходными позициями и CLI.

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

## Не входит в Stage 1

Canonical IR, разрешение `derivedFrom`, раскрытие `dim`, наследование
свойств регистров, проверки пересечений полей и адресных диапазонов, полная
XSD-валидация, SARIF, автоисправления, GUI.

Готовые точки расширения для Stage 2: `SourceMap`/`NodeId`/`SourceId` для
связывания IR с XML-узлами, неизменная модель `Diagnostic` со стабильными
кодами, `FrontendResult` как вход нормализации.
