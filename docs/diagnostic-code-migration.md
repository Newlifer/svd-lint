# Миграция кодов диагностик

Новые коды обозначают фиксированную severity: `E001–E999` — error, `W001–W999` — warning, `N001–N999` — note. Нумерация независима для каждого префикса. У каждого кода один неизменный уровень. Профили и конфигурация могут отключать проверки, но не менять severity. Несовпадающий уровень в конфигурации считается ошибкой.

Нужно обновить коды в `svd-lint.toml`, скриптах и интеграциях; старые идентификаторы больше не принимаются. Ранее сохранённые внешние отчёты содержат исторические коды и не пересоздаются этой миграцией. Отдельных диагностик note в текущем каталоге нет.

| Старый код | Новый код | Диагностика |
|---|---|---|
| SVD007 | W001 | low-quality-description |
| IO001 | E001 | io-read-failed |
| IO002 | E002 | io-invalid-utf8 |
| XML001 | E003 | xml-syntax |
| SVD001 | E004 | invalid-root |
| SVD002 | E005 | missing-required-element |
| SVD003 | E006 | duplicate-singleton |
| SVD004 | E007 | empty-required-element |
| SVD005 | W002 | missing-peripheral-description |
| SVD006 | E008 | typed-parse-failed |
| NORM001 | E009 | missing-reference |
| NORM002 | E010 | reference-cycle |
| NORM003 | E011 | ambiguous-reference |
| NORM004 | E012 | invalid-dimension |
| NORM005 | E013 | invalid-array-index |
| NORM006 | E014 | invalid-array-template |
| NORM007 | E015 | address-overflow |
| NORM008 | E016 | missing-register-size |
| NORM009 | E017 | expansion-limit |
| SEM001 | E018 | zero-register-size |
| SEM002 | E019 | zero-field-width |
| SEM003 | E020 | field-out-of-bounds |
| SEM004 | W003 | overlapping-fields |
| SEM005 | E021 | reset-value-out-of-bounds |
| SEM006 | E022 | reset-mask-out-of-bounds |
| SEM007 | W004 | reset-outside-mask |
| SEM008 | E023 | overlapping-registers |
| SEM009 | E024 | invalid-alternate-register |
| SEM010 | E025 | register-outside-block |
| SEM011 | E026 | invalid-address-block |
| SEM012 | E027 | enum-out-of-bounds |
| SEM013 | E028 | duplicate-enum-entry |
| SEM014 | E029 | multiple-enum-defaults |
| SEM015 | E030 | invalid-write-constraint |
| SEM016 | W005 | access-conflict |
| XSD001 | E031 | unknown-schema-version |
| XSD002 | E032 | schema-nonconformance |
| XSD003 | E033 | schema-not-found |
| XSD004 | E034 | schema-backend-failed |

Предупреждения, ранее использовавшие коды ошибок, выделены отдельно:

| Старый код | Код ошибки | Код предупреждения |
|---|---|---|
| SEM008 | E023 — подтверждённое пересечение | W006 — возможные альтернативные представления |
| SEM013 | E028 — конфликтующие значения одного имени | W007 — избыточные записи или описания одного значения |
