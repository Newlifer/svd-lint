# svd-lint

Статический анализатор CMSIS-SVD файлов.

## Использование

```console
$ svd-lint check path/to/device.svd
$ svd-lint check path/to/device.svd --format json
$ svd-lint dump-ir path/to/device.svd --format json
$ svd-lint list-rules --format json
$ svd-lint explain SEM004
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

## Структура

```text
crates/
├── svd-lint-core/   # библиотека: SourceFile, SourceMap, preflight, parse_svd, диагностики
└── svd-lint-cli/    # бинарник svd-lint (clap + miette + JSON)
tests/fixtures/      # общие SVD-фикстуры (valid/ и invalid/)
```