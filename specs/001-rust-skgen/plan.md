# Plan técnico: Generación y actualización de skills

## Alcance y trazabilidad

Este plan implementa exclusivamente la especificación 001. La creación, protección frente a sobrescritura, descubrimiento, contenido, actualización y resultado se cubren respectivamente en RF-1, RF-2, RF-3, RF-4, RF-5 y RF-6, y RF-7.

## Estructura de módulos

| Módulo | Responsabilidad | RF cubiertos |
| --- | --- | --- |
| `cli` | Analiza comandos, valida combinaciones de argumentos, solicita confirmaciones y muestra resultados. No contiene red, parseo ni escritura. | RF-1, RF-5, RF-6, RF-7 |
| `domain` | Define los tipos validados: nombre de skill, URL de origen, configuración de descubrimiento, formato, metadatos, skill normalizada y resultados. | RF-1, RF-3, RF-4, RF-6 |
| `fetch` | Obtiene recursos HTTP(S), sin seguir redirecciones, aplica `User-Agent`, tiempos de espera, reintentos acotados y concurrencia máxima de una solicitud. | RF-1, RF-3 |
| `policy` | Descarga y evalúa `robots.txt`, valida el alcance de cada URL y decide si una página puede visitarse. | RF-3 |
| `discover` | Recorre enlaces, elimina duplicados por URL sin consulta ni fragmento, aplica alcance y modo de recorrido, y devuelve páginas ordenadas. | RF-3 |
| `extract` | Convierte HTML extraído en páginas documentales normalizadas con URL y contenido atribuible. | RF-3, RF-4 |
| `render` | Genera de forma determinista los formatos de guía con referencias y contenido organizado a partir del modelo normalizado. | RF-4 |
| `metadata` | Serializa, valida y compara metadatos de gestión y la huella del contenido generado. | RF-4, RF-5, RF-6 |
| `storage` | Localiza skills, aplica bloqueo exclusivo, crea, reemplaza, renombra y restaura resultados de forma transaccional. | RF-1, RF-2, RF-5, RF-6 |
| `service` | Orquesta crear y actualizar mediante interfaces de los módulos anteriores; devuelve resultados por skill. | RF-1 a RF-7 |

La dependencia entre módulos fluye hacia el modelo de dominio. `cli` llama a `service`; `service` usa los puertos de red, política, descubrimiento, renderizado y almacenamiento. Así se mantiene la separación exigida por la constitución.

## Modelo de datos JSON

Los metadatos son la persistencia mínima autorizada por la especificación: identifican una skill gestionada, permiten recrearla y detectan modificaciones manuales del contenido generado. **Cubre RF-4, RF-5 y RF-6.**

```json
{
  "schema_version": 1,
  "generator": "rust-skgen",
  "source_url": "https://www.radix-ui.com/primitives/docs/overview/introduction",
  "discovery": {
    "scope": "same-site",
    "site_boundary": "exact-host",
    "allowed_subdomains": [],
    "traversal": {
      "mode": "all"
    },
    "require_robots_txt": false
  },
  "content_format": "guide-with-references",
  "content_digest": "sha256:7b50fcd0d5f3a4c8b3e53b7a8585a35e42f2cfc0cb37360e6f4ecaa7f2e7166e"
}
```

Valores permitidos:

- `scope`: `same-site`, `path-prefix`, `parent-directory`, `documentation-navigation`.
- `site_boundary`: `exact-host`, `base-domain`, `same-origin`; solo aplica a `same-site`.
- `traversal.mode`: `all`, `one-level`, `limited`; `limited` añade `max_pages`, entero positivo, con valor predeterminado `100`.
- `require_robots_txt`: `true` exige obtener y validar `robots.txt`; `false` permite continuar cuando no está disponible o no es válido. El valor predeterminado es `false`.
- `content_format`: `guide-with-references` o `organized-content`.

La huella se calcula sobre el contenido gestionado renderizado de forma determinista. Una discrepancia, un metadato inválido o ausente obliga a solicitar la reconstrucción antes de actualizar. **Cubre RF-4 y RF-6.**

## Algoritmo de lectura del sitio

**Cubre RF-1, RF-2, RF-3, RF-4 y RF-7.**

```text
function build_skill(configuration, previous_skill):
    validate name, URL scheme, configuration combinations and max_pages
    reject if the source URL or any fetched page redirects
    acquire exclusive lock for the target skill
    prepare an isolated pending result

    fetch and evaluate robots policy for the source URL
    if robots.txt is valid: enforce its applicable rules
    if robots.txt is missing, inaccessible, unsuccessful, or invalid:
        fail if require_robots_txt is true
        otherwise continue discovery
    fetch source URL; fail if inaccessible, forbidden, non-documental or redirected
    add source page to the pending set unconditionally

    queue = links discovered from source page
    visited = { canonicalize(source URL) }
    while queue is not empty and traversal may continue:
        candidate = next URL in deterministic URL order
        canonical = remove query and fragment from candidate
        skip if canonical is visited
        mark canonical as visited
        skip if candidate is outside the selected scope
        skip if base-domain candidate is not both explicitly authorized and linked
        fetch and evaluate robots policy for candidate
        fail this skill if robots.txt is required but unavailable or invalid
        fail this skill if the candidate is forbidden by valid robots.txt, inaccessible or redirected
        extract its document content; fail this skill if it is not documental
        add page to pending set
        if traversal is all: enqueue its links
        if traversal is one-level: do not enqueue its links
        if traversal is limited: enqueue links until max_pages is reached

    fail if pending set has no valid documentation page
    render the selected content format from pages sorted by canonical URL
    derive guide objective and instructions only from extracted content
    create metadata and content digest
    atomically publish the pending result:
        create: fail if target already exists
        update: replace prior managed content only after the pending result is complete
    on any failure: discard pending result and preserve the prior skill unchanged
    release lock and return the per-skill result
```

El modo `limited` cuenta la página inicial dentro de `max_pages`; al alcanzar el límite no es un error y se publica lo ya extraído. El modo `all` y la cola ordenada garantizan resultados repetibles para las mismas fuentes. Si una operación de lote llama a este algoritmo para varias skills, continúa tras cada fallo individual. **Cubre RF-3, RF-4, RF-6 y RF-7.**

## Contrato de la CLI

Los nombres de comandos y los mensajes visibles se escriben en inglés. **Cubre RF-1, RF-3, RF-5, RF-6 y RF-7.**

```text
rust-skgen create <source-url> <skill-name>
  [--scope same-site|path-prefix|parent-directory|documentation-navigation]
  [--site-boundary exact-host|base-domain|same-origin]
  [--allow-subdomain <host>]...
  [--traversal all|one-level|limited]
  [--max-pages <positive-integer>]
  [--format guide-with-references|organized-content]
  [--require-robots-txt true|false]
  [--user-agent <value>]

rust-skgen update [<skill-name>...]
  [--name <skill-name>]
  [--source-url <source-url>]
  [--scope same-site|path-prefix|parent-directory|documentation-navigation]
  [--site-boundary exact-host|base-domain|same-origin]
  [--allow-subdomain <host>]...
  [--traversal all|one-level|limited]
  [--max-pages <positive-integer>]
  [--format guide-with-references|organized-content]
  [--require-robots-txt true|false]
  [--user-agent <value>]
```

- `create` exige URL HTTP(S) sin redirección y un slug válido. Crea solo bajo `.agents/skills/<skill-name>` relativo al directorio actual.
- Los valores predeterminados son `same-site`, `exact-host`, `all`, `guide-with-references` y `require-robots-txt=false`; `limited` usa `100` páginas si se omite `--max-pages`.
- `--site-boundary` y `--allow-subdomain` se rechazan si el alcance no es `same-site`. Un subdominio adicional requiere simultáneamente `base-domain`, `--allow-subdomain` y un enlace desde una página ya incluida.
- Las reglas de un `robots.txt` válido se respetan siempre. Si no existe, no se puede obtener o no es válido, el descubrimiento continúa salvo que `--require-robots-txt true` lo exija; en ese caso la skill falla sin cambios parciales.
- `update` sin nombres actualiza todas las skills con metadatos válidos de la CLI. Sin skills gestionadas, informa `No managed skills found.`
- `update` con un nombre permite cambiar cualquier configuración, incluida la exigencia de `robots.txt`, y conserva esos valores en los metadatos. Con varios nombres, cualquier parámetro de cambio rechaza la operación completa antes de actualizar.
- Si faltan metadatos, son inválidos o no coinciden con la huella de contenido, la CLI pregunta `Rebuild metadata and update this skill? [y/N]`. La ausencia de respuesta o una respuesta negativa conserva la skill sin cambios.
- Una modificación manual no bloquea la actualización si los metadatos y la huella son válidos; el contenido generado se reemplaza.

Salidas de éxito, en salida estándar:

```text
Created skill: <skill-name>
Updated skill: <skill-name>
No managed skills found.
```

Fallos, en salida de error:

```text
Failed skill: <skill-name>: <reason>
Invalid argument: <reason>
```

| Código | Significado |
| --- | --- |
| `0` | La operación terminó sin skills fallidas, incluido el caso sin skills gestionadas. |
| `1` | Una o más skills fallaron, incluidos bloqueo, acceso denegado, redirección, extracción o escritura. |
| `2` | Argumentos inválidos o combinación de argumentos no permitida; no se procesa ninguna skill. |

## Decisiones técnicas

| Decisión | Justificación | Alternativa descartada |
| --- | --- | --- |
| Rust estable y Cargo | Cumple el principio de simplicidad y el marco del proyecto. | Toolchains nocturnos o gestores de paquetes alternativos. |
| Cliente HTTP bloqueante, secuencial y con concurrencia máxima de una | Satisface la limitación de concurrencia, simplifica el respeto de límites y facilita pruebas deterministas. | Ejecución asíncrona y paralela, innecesaria para el MVP. |
| Dependencia HTTP con soporte HTTPS, control de redirecciones, cabeceras y tiempos de espera | La biblioteca estándar no ofrece cliente HTTPS ni estas garantías. | Implementar HTTP/HTTPS manualmente con sockets. |
| Analizador HTML estructurado | Permite extraer enlaces y elementos de navegación sin interpretar HTML con expresiones regulares. | Expresiones regulares sobre HTML. |
| Biblioteca de URLs | Permite normalizar, resolver enlaces relativos, aplicar alcance y eliminar consulta y fragmento correctamente. | Manipulación de URLs mediante cadenas. |
| Serialización JSON y suma de verificación criptográfica | Permite validar la gestión de una skill y detectar cambios antes de reconstruir metadatos. | Formato ad hoc sin validación o confiar solo en la existencia de una carpeta. |
| Escritura transaccional con resultado pendiente, reemplazo y restauración | Cumple la obligación de no dejar contenido parcial ni modificar una versión previa ante fallo. | Escritura directa sobre la skill existente. |
| Modelo normalizado antes de renderizar | Permite los dos formatos y una salida determinista sin mezclar extracción con presentación. | Renderizar durante el parseo de cada página. |

Las dependencias propuestas quedan justificadas por los requisitos de HTTP(S), parseo HTML, URLs y metadatos JSON. Antes de incorporarlas al manifiesto de dependencias se debe reflejar esa decisión en la especificación activa, conforme a las reglas del proyecto.

## Estrategia de tests

Todos los nombres de tests y sus mensajes estarán en inglés; los datos de prueba serán locales y deterministas. **Cubre RF-1 a RF-7 y los requisitos no funcionales.**

| Nivel | Casos | RF cubiertos |
| --- | --- | --- |
| Unidad de dominio | Slugs válidos e inválidos, esquemas HTTP(S), valores predeterminados, combinaciones de opciones y máximo de páginas. | RF-1, RF-3, RF-6 |
| Unidad de alcance | Host y puerto exactos, mismo origen, dominio base con autorización y enlace, prefijo, directorio padre, navegación y URLs equivalentes. | RF-3 |
| Unidad de renderizado y metadatos | Orden estable, ambos formatos, atribución, contenido derivado de fuentes, serialización, validación y discrepancia de huella. | RF-4, RF-6 |
| Integración de red local | Respuestas HTTP correctas, redirecciones, errores HTTP, HTML no documental, límites de tiempo, reintentos, `robots.txt` permitido y prohibido, y ausencia, inaccesibilidad o contenido inválido con exigencia opcional u obligatoria. | RF-1, RF-3 |
| Integración de almacenamiento temporal | Creación, conflicto de nombre, escritura fallida, reemplazo, renombrado, restauración y bloqueo simultáneo. | RF-1, RF-2, RF-5, RF-6 |
| Integración de servicio | Actualización sin argumentos, selección válida y mixta, lotes que continúan tras fallo, rechazo de cambios masivos y reconstrucción aceptada o rechazada. | RF-5, RF-6, RF-7 |
| Pruebas de CLI | Argumentos, exigencia de `robots.txt`, mensajes en inglés, confirmación interactiva, salida estándar, salida de error y códigos `0`, `1` y `2`. | RF-1, RF-3, RF-5, RF-6, RF-7 |
| Regresión | Una prueba determinista por corrección de error público. | RF-1 a RF-7 |

La validación final ejecutará `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings` y `cargo test` cuando el proyecto sea compilable.
